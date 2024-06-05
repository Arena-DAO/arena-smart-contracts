use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub enum NestedArray<T> {
    Single(Vec<T>),
    Nested(Vec<NestedArray<T>>),
}

impl<T: Copy> NestedArray<T> {
    pub fn nest(self) -> NestedArray<T> {
        // Box the initial NestedArray to start the process on the heap
        let mut current = Box::new(self);

        loop {
            match *current {
                NestedArray::Single(vec) => {
                    let len = vec.len();
                    if len == 2 {
                        return NestedArray::Single(vec);
                    }
                    let mut nested_pairs = Vec::with_capacity(len / 2);

                    for i in 0..(len / 2) {
                        nested_pairs.push(NestedArray::Single(vec![vec[i], vec[len - 1 - i]]));
                    }

                    // Recursively nest these pairs, managed on the heap
                    current = Box::new(NestedArray::Nested(nested_pairs));
                }
                NestedArray::Nested(nested_vec) => {
                    let len = nested_vec.len();
                    if len == 2 {
                        return NestedArray::Nested(nested_vec);
                    }
                    let mut nested_pairs = Vec::with_capacity(len / 2);

                    for i in 0..(len / 2) {
                        nested_pairs.push(NestedArray::Nested(vec![
                            nested_vec[i].clone(),
                            nested_vec[len - 1 - i].clone(),
                        ]));
                    }

                    // Continue nesting on the heap
                    current = Box::new(NestedArray::Nested(nested_pairs));
                }
            }
        }
    }

    // This method nests the array and immediately flattens it, skipping the intermediate nested structure
    pub fn nest_flat(self) -> VecDeque<T> {
        let mut result = VecDeque::new();
        let mut current = Box::new(self);

        loop {
            match *current {
                NestedArray::Single(vec) => {
                    let len = vec.len();
                    if len <= 2 {
                        result.extend(vec);
                        return result;
                    }
                    let mut nested_pairs = Vec::with_capacity(len / 2);

                    for i in 0..(len / 2) {
                        nested_pairs.push(NestedArray::Single(vec![vec[i], vec[len - 1 - i]]));
                    }

                    // Continue processing without returning to user
                    current = Box::new(NestedArray::Nested(nested_pairs));
                }
                NestedArray::Nested(nested_vec) => {
                    let len = nested_vec.len();
                    if len <= 2 {
                        for nested in nested_vec {
                            nested.flatten_into(&mut result);
                        }
                        return result;
                    }
                    let mut nested_pairs = Vec::with_capacity(len / 2);

                    for i in 0..(len / 2) {
                        nested_pairs.push(NestedArray::Nested(vec![
                            nested_vec[i].clone(),
                            nested_vec[len - 1 - i].clone(),
                        ]));
                    }

                    // Continue processing without returning to user
                    current = Box::new(NestedArray::Nested(nested_pairs));
                }
            }
        }
    }

    fn flatten_into(self, result: &mut VecDeque<T>) {
        match self {
            NestedArray::Single(vec) => {
                result.extend(vec);
            }
            NestedArray::Nested(vec) => {
                for nested in vec {
                    nested.flatten_into(result);
                }
            }
        }
    }
}
