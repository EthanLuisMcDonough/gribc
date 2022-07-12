pub(in runtime::memory) mod heap;
mod mark;
pub(in runtime::memory) mod slot;
pub(in runtime::memory) mod stack;

pub use self::heap::Gc;
pub use self::slot::StackSlot;
pub use self::stack::Stack;

use self::mark::*;
use ast::node::{Param, Parameters, StackPointer};
use runtime::memory::slot::*;
use runtime::values::{GribString, GribValue, HeapValue};

pub struct RuntimeConfig {
    pub cleanup_after: usize,
}

pub struct Runtime {
    pub gc: Gc,
    pub stack: Stack,
    free_pointers: Vec<usize>,
    allocations: usize,
    max_allocations: usize,
}

impl Runtime {
    pub fn new(config: RuntimeConfig) -> Self {
        Self {
            gc: Gc::new(),
            stack: Stack::new(),
            free_pointers: Vec::new(),
            allocations: 0,
            max_allocations: config.cleanup_after,
        }
    }

    pub fn clean(&mut self) {
        for slot in self.stack.iter() {
            match slot {
                StackSlot::Captured(ind) => mark_heap(&mut self.gc, *ind),
                StackSlot::Value(val) => mark_stack(&mut self.gc, val),
            }
        }

        let len = self.gc.heap.len();
        for index in 0..len {
            if !self.gc.heap[index].marked {
                self.gc.remove(index);
                self.free_pointers.push(index);
            }

            self.gc.heap[index].marked = false;
        }
    }

    pub fn read_slot(&self, ptr: StackPointer) -> &StackSlot {
        match ptr {
            StackPointer::StackOffset(offset) => self.stack.offset_slot(offset),
            StackPointer::CaptureIndex(index) => self
                .stack
                .get_call_stack()
                .and_then(|i| self.gc.try_get_stack(i))
                .and_then(|stack| stack.get(index)),
        }
        .unwrap_or_else(|| panic!("FAILED TO READ POINTER: {:?}", ptr))
    }

    pub fn read_slot_mut(&mut self, ptr: StackPointer) -> &mut StackSlot {
        match ptr {
            StackPointer::StackOffset(offset) => self.stack.offset_slot_mut(offset),
            StackPointer::CaptureIndex(index) => self
                .stack
                .get_call_stack()
                .and_then(move |i| self.gc.try_get_stack_mut(i))
                .and_then(|stack| stack.get_mut(index)),
        }
        .unwrap_or_else(|| panic!("FAILED TO READ POINTER: {:?}", ptr))
    }

    pub fn read_val(&self, ptr: StackPointer) -> &GribValue {
        self.read_slot(ptr).get(&self.gc)
    }

    pub fn read_val_mut(&mut self, ptr: StackPointer) -> &mut GribValue {
        match ptr {
            StackPointer::StackOffset(offset) => {
                let gc = &mut self.gc;
                self.stack
                    .offset_slot_mut(offset)
                    .map(move |slot| slot.get_mut(gc))
            }
            StackPointer::CaptureIndex(index) => self
                .stack
                .get_call_stack()
                .and_then(|i| self.gc.try_get_stack(i))
                .and_then(|stack| stack.get(index))
                .cloned()
                .and_then(move |slot| match slot {
                    StackSlot::Captured(i) => Some(self.gc.get_captured_mut(i)),
                    _ => None,
                }),
        }
        .unwrap_or_else(|| panic!("FAILED TO READ POINTER: {:?}", ptr))
    }

    fn alloc(&mut self, value: impl Into<Option<HeapSlot>>) -> usize {
        let value = Markable {
            value: value.into(),
            marked: false,
        };

        if self.allocations > self.max_allocations {
            self.clean();
            self.allocations = 0;
        }

        self.allocations += 1;

        if let Some(index) = self.free_pointers.pop() {
            self.gc.heap[index] = value;
            index
        } else {
            let index = self.gc.heap.len();
            self.gc.heap.push(value);
            index
        }
    }

    pub fn alloc_heap(&mut self, value: HeapValue) -> usize {
        self.alloc(HeapSlot::Value(value))
    }

    pub fn reserve_slot(&mut self) -> usize {
        self.alloc(None)
    }

    pub fn alloc_captured(&mut self, value: GribValue) -> usize {
        self.alloc(HeapSlot::Captured(value))
    }

    pub fn add_stack_captured(&mut self, value: GribValue) {
        let ind = self.alloc_captured(value);
        self.stack.add(StackSlot::Captured(ind));
    }

    pub fn alloc_str(&mut self, s: String) -> GribString {
        if s.is_empty() {
            GribString::Static("")
        } else if s.len() == 1 {
            GribString::Char(s.chars().next().unwrap())
        } else {
            GribString::Heap(self.alloc_heap(HeapValue::String(s)))
        }
    }

    pub fn capture_stack(&mut self, to_capture: &Vec<StackPointer>) -> Option<usize> {
        if to_capture.is_empty() {
            return None;
        }

        let mut heap_stack = Vec::with_capacity(to_capture.len());
        for &ptr in to_capture {
            heap_stack.push(self.read_slot(ptr).clone());
        }

        self.alloc_heap(HeapValue::CapturedStack(heap_stack)).into()
    }

    pub fn add_stack(&mut self, stack_ind: impl Into<Option<usize>>) -> usize {
        let stack_ind = stack_ind.into();
        let mut allocated = 0usize;
        if let Some(HeapValue::CapturedStack(stack)) =
            stack_ind.and_then(|ind| self.gc.heap_val(ind)).cloned()
        {
            allocated = stack.len();
            for slot in stack {
                self.stack.add(slot);
            }
        }
        allocated
    }

    pub fn add_param(&mut self, param: &Param, val: GribValue) {
        if param.captured {
            self.add_stack_captured(val);
        } else {
            self.stack.add(val);
        }
    }

    pub fn add_params(&mut self, params: &Parameters, args: Vec<GribValue>) -> usize {
        let mut arg_iter = args.into_iter();
        let mut alloced = params.params.len();

        for param in &params.params {
            self.add_param(param, arg_iter.next().unwrap_or_default());
        }

        if let Some(spread) = &params.vardic {
            let args = HeapValue::Array(arg_iter.collect());
            let val = GribValue::HeapValue(self.alloc_heap(args));
            self.add_param(spread, val);
            alloced += 1;
        }

        alloced
    }
}
