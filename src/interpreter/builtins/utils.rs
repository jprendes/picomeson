use crate::interpreter::Value;

pub fn flatten<'a>(args: &'a (impl AsValueSlice<'a> + ?Sized)) -> Flatten<'a> {
    Flatten::new(args)
}

pub struct Flatten<'a> {
    args_stack: Vec<&'a [Value]>,
}

pub trait AsValueSlice<'a> {
    fn as_slice(&'a self) -> &'a [Value];
}

impl<'a> AsValueSlice<'a> for Vec<Value> {
    fn as_slice(&'a self) -> &'a [Value] {
        self.as_ref()
    }
}

impl<'a> AsValueSlice<'a> for [Value] {
    fn as_slice(&'a self) -> &'a [Value] {
        self
    }
}

impl<'a> AsValueSlice<'a> for Option<&Value> {
    fn as_slice(&'a self) -> &'a [Value] {
        match *self {
            Some(v) => std::slice::from_ref(v),
            None => &[],
        }
    }
}

impl<'a> Flatten<'a> {
    fn new<'b: 'a>(args: &'b (impl AsValueSlice<'a> + ?Sized)) -> Flatten<'a> {
        Flatten {
            args_stack: vec![args.as_slice()],
        }
    }
}

impl<'a> Iterator for Flatten<'a> {
    type Item = &'a Value;

    fn next(&mut self) -> Option<Self::Item> {
        let args = self.args_stack.pop()?;

        if args.is_empty() {
            return self.next();
        }

        let (first, rest) = args.split_first().unwrap();

        if !rest.is_empty() {
            self.args_stack.push(rest);
        }

        if let Value::Array(arr) = first {
            self.args_stack.push(arr);
            self.next()
        } else {
            Some(first)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_flatten() {
        let input = vec![
            Value::Integer(1),
            Value::Array(vec![
                Value::Array(vec![]),
                Value::Integer(2),
                Value::Array(vec![Value::Integer(3), Value::Integer(4)]),
                Value::Array(vec![]),
            ]),
            Value::Array(vec![]),
            Value::Integer(5),
        ];
        let expected = vec![
            &Value::Integer(1),
            &Value::Integer(2),
            &Value::Integer(3),
            &Value::Integer(4),
            &Value::Integer(5),
        ];
        let flattened = flatten(&input).collect::<Vec<_>>();
        assert_eq!(flattened, expected);
    }
}
