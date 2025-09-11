#![allow(dead_code)]

use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use hashbrown::HashMap;

use crate::parser::{BinaryOperator, ParseError, Parser, Statement, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum MachineValue {
    String(String),
    Integer(i64),
    Boolean(bool),
    Array(Vec<MachineValue>),
}

impl MachineValue {
    /// Convert from parser Value to MachineValue
    fn from_value(value: Value) -> Result<Self, ParseError> {
        match value {
            Value::String(s) => Ok(MachineValue::String(s)),
            Value::Integer(i) => Ok(MachineValue::Integer(i)),
            Value::Boolean(b) => Ok(MachineValue::Boolean(b)),
            Value::Array(arr) => {
                let mut result = Vec::new();
                for item in arr {
                    result.push(MachineValue::from_value(item)?);
                }
                Ok(MachineValue::Array(result))
            }
            _ => Err(ParseError::UnexpectedToken),
        }
    }

    /// Convert MachineValue back to Value for evaluation
    fn to_value(&self) -> Value {
        match self {
            MachineValue::String(s) => Value::String(s.clone()),
            MachineValue::Integer(i) => Value::Integer(*i),
            MachineValue::Boolean(b) => Value::Boolean(*b),
            MachineValue::Array(arr) => Value::Array(arr.iter().map(|v| v.to_value()).collect()),
        }
    }
}

impl MachineValue {
    pub fn as_string(&self) -> Option<&str> {
        if let MachineValue::String(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn coerce_string(&self) -> String {
        match self {
            MachineValue::String(s) => s.clone(),
            MachineValue::Integer(i) => i.to_string(),
            MachineValue::Boolean(b) => b.to_string(),
            MachineValue::Array(arr) => {
                let strs: Vec<String> = arr.iter().map(|v| v.coerce_string()).collect();
                strs.join(",")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MachineFile {
    pub sections: HashMap<String, HashMap<String, MachineValue>>,
}

impl MachineFile {
    fn new() -> Self {
        Self::default()
    }

    pub fn parse(content: &str) -> Result<MachineFile, ParseError> {
        let mut parser = MachineFileParser::new(content);
        let machine_file = parser.parse()?;
        Ok(machine_file)
    }

    /// Get a value from a section
    pub fn get(&self, section: &str, key: &str) -> Option<&MachineValue> {
        self.sections.get(section)?.get(key)
    }

    /// Set a value in a section
    fn set(&mut self, section: &str, key: &str, value: MachineValue) {
        self.sections
            .entry(section.to_string())
            .or_default()
            .insert(key.to_string(), value);
    }

    pub fn section(&self, section: &str) -> Option<&HashMap<String, MachineValue>> {
        self.sections.get(section)
    }
}

struct MachineFileParser {
    lines: Vec<String>,
    pos: usize,
    sections: HashMap<String, HashMap<String, (usize, Value)>>,
}

impl MachineFileParser {
    fn new(content: &str) -> Self {
        let lines = content
            .lines()
            .map(|line| line.trim().to_string())
            .collect();

        Self {
            lines,
            pos: 0,
            sections: HashMap::new(),
        }
    }

    fn set(&mut self, section: &str, key: &str, value: Value) {
        let section = self.sections.entry(section.to_string()).or_default();

        let n = section.len();
        if let Some(entry) = section.get_mut(key) {
            // Key already exists, overwrite it
            entry.1 = value;
        } else {
            section.insert(key.to_string(), (n, value));
        }
    }

    fn parse(&mut self) -> Result<MachineFile, ParseError> {
        self.pos = 0;

        let mut current_section = String::new();

        while self.pos < self.lines.len() {
            let line = &self.lines[self.pos].clone();
            self.pos += 1;

            // Skip empty lines
            if line.is_empty() {
                continue;
            }

            // Check for section header
            if line.starts_with('[') && line.ends_with(']') {
                current_section = line[1..line.len() - 1].to_string();
                continue;
            }

            // Parse key-value assignment
            if let Some(equals_pos) = line.find('=') {
                let key = line[..equals_pos].trim().to_string();
                let value_str = line[equals_pos + 1..].trim();

                // Parse the value expression with available variables
                let value = self.parse_value(value_str)?;

                // Add to section variables for future references
                self.set(&current_section, &key, value);
            }
        }

        let mut machine_file = MachineFile::new();

        let mut sections = core::mem::take(&mut self.sections);

        // Evaluate all values with context of their section variables
        if let Some(entries) = sections.remove("constants") {
            self.evaluate_section(&mut machine_file, "constants", entries)?;
        }

        for (section, entries) in sections.into_iter() {
            self.evaluate_section(&mut machine_file, &section, entries)?;
        }

        Ok(machine_file)
    }

    fn evaluate_section(
        &self,
        machine_file: &mut MachineFile,
        section: &str,
        entries: HashMap<String, (usize, Value)>,
    ) -> Result<(), ParseError> {
        let mut entries = entries.into_iter().collect::<Vec<_>>();
        entries.sort_by_key(|(_, (idx, _))| *idx);
        for (k, (_, v)) in entries {
            let evaluated = machine_file.evaluate_value(section, &v)?;
            let mv = MachineValue::from_value(evaluated)?;
            machine_file.set(section, &k, mv);
        }
        Ok(())
    }

    fn parse_value(&mut self, first_line: &str) -> Result<Value, ParseError> {
        let mut array_content = String::from(first_line);

        // Continue reading lines until we find the closing bracket
        while self.pos < self.lines.len() {
            // Use the main parser to decide when the statement is complete
            if Parser::new(&array_content).parse().is_ok() {
                break;
            }

            let line = self.lines[self.pos].as_str();
            self.pos += 1;

            array_content.push('\n');
            array_content.push_str(line);
        }

        // Parse the complete value
        let mut parser = Parser::new(&array_content);
        let mut statements = parser.parse()?;

        if statements.len() != 1 {
            return Err(ParseError::UnexpectedToken);
        }

        if let Statement::Expression(expr) = statements.swap_remove(0) {
            Ok(expr)
        } else {
            Err(ParseError::UnexpectedToken)
        }
    }
}

impl MachineFile {
    fn evaluate_value(&self, section: &str, value: &Value) -> Result<Value, ParseError> {
        match value {
            Value::String(s) => Ok(Value::String(s.clone())),
            Value::Integer(i) => Ok(Value::Integer(*i)),
            Value::Boolean(b) => Ok(Value::Boolean(*b)),
            Value::Array(arr) => {
                let mut result = Vec::new();
                for item in arr {
                    result.push(self.evaluate_value(section, item)?);
                }
                Ok(Value::Array(result))
            }
            Value::Identifier(name) => {
                // Look up variable in constants first, then section variables
                if let Some(val) = self.get("constants", name) {
                    Ok(val.to_value())
                } else if let Some(val) = self.get(section, name) {
                    Ok(val.to_value())
                } else {
                    Err(ParseError::UnexpectedToken)
                }
            }
            Value::BinaryOp(left, op, right) => {
                let left_val = self.evaluate_value(section, left)?;
                let right_val = self.evaluate_value(section, right)?;

                match op {
                    BinaryOperator::Add => {
                        // String/array concatenation
                        match (&left_val, &right_val) {
                            (Value::String(a), Value::String(b)) => {
                                let mut result = a.clone();
                                result.push_str(b);
                                Ok(Value::String(result))
                            }
                            (Value::Array(a), Value::Array(b)) => {
                                let mut result = a.clone();
                                result.extend(b.clone());
                                Ok(Value::Array(result))
                            }
                            (Value::Array(a), Value::String(b)) => {
                                let mut result = a.clone();
                                result.push(Value::String(b.clone()));
                                Ok(Value::Array(result))
                            }
                            (Value::String(a), Value::Array(b)) => {
                                let mut result = vec![Value::String(a.clone())];
                                result.extend(b.iter().cloned());
                                Ok(Value::Array(result))
                            }
                            _ => Err(ParseError::UnexpectedToken),
                        }
                    }
                    BinaryOperator::Div => {
                        // Path joining
                        match (&left_val, &right_val) {
                            (Value::String(a), Value::String(b)) => {
                                // TODO: use the Os::join_paths method here
                                let mut result = a.clone();
                                if !result.ends_with('/') && !result.ends_with('\\') {
                                    result.push('/');
                                }
                                // Remove leading slash from right side if present
                                let b = if b.starts_with('/') || b.starts_with('\\') {
                                    &b[1..]
                                } else {
                                    b
                                };
                                result.push_str(b);
                                Ok(Value::String(result))
                            }
                            _ => Err(ParseError::UnexpectedToken),
                        }
                    }
                    _ => Err(ParseError::UnexpectedToken),
                }
            }
            _ => Err(ParseError::UnexpectedToken),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ... existing tests ...

    #[test]
    fn test_constants_section() {
        let content = r##"
[constants]
toolchain_prefix = '/usr/bin/'
gcc_name = 'gcc'
base_url = 'https://example.com'
url = base_url / '#some_hash'
array = [
    'item1',
    'item2',
    'item3'
] + 'item4'

[binaries]
c = toolchain_prefix / gcc_name # some comment here
cpp = toolchain_prefix / 'g++'
"##;
        let machine_file = MachineFile::parse(content).unwrap();

        assert_eq!(
            machine_file.get("constants", "array"),
            Some(&MachineValue::Array(vec![
                MachineValue::String("item1".to_string()),
                MachineValue::String("item2".to_string()),
                MachineValue::String("item3".to_string()),
                MachineValue::String("item4".to_string()),
            ]))
        );
        assert_eq!(
            machine_file.get("constants", "url"),
            Some(&MachineValue::String(
                "https://example.com/#some_hash".to_string()
            ))
        );
        assert_eq!(
            machine_file.get("binaries", "c"),
            Some(&MachineValue::String("/usr/bin/gcc".to_string()))
        );
        assert_eq!(
            machine_file.get("binaries", "cpp"),
            Some(&MachineValue::String("/usr/bin/g++".to_string()))
        );
    }

    #[test]
    fn test_string_concatenation() {
        let content = r#"
[constants]
prefix = '-D'
foo = 'FOO'

[properties]
c_args = [prefix + foo + '=1', '-DBAR=2']
"#;
        let machine_file = MachineFile::parse(content).unwrap();

        if let Some(MachineValue::Array(arr)) = machine_file.get("properties", "c_args") {
            assert_eq!(arr[0], MachineValue::String("-DFOO=1".to_string()));
        } else {
            panic!("Expected array value");
        }
    }

    #[test]
    fn test_array_concatenation() {
        let content = r#"
[constants]
base_args = ['-O2', '-g']

[properties]
c_args = base_args + ['-DFOO=1']
"#;
        let machine_file = MachineFile::parse(content).unwrap();

        if let Some(MachineValue::Array(arr)) = machine_file.get("properties", "c_args") {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], MachineValue::String("-O2".to_string()));
            assert_eq!(arr[1], MachineValue::String("-g".to_string()));
            assert_eq!(arr[2], MachineValue::String("-DFOO=1".to_string()));
        } else {
            panic!("Expected array value");
        }
    }

    #[test]
    fn test_section_local_variables() {
        let content = r#"
[binaries]
prefix = '/usr/bin/'
c = prefix / 'gcc'
cpp = prefix / 'g++'
"#;
        let machine_file = MachineFile::parse(content).unwrap();

        assert_eq!(
            machine_file.get("binaries", "c"),
            Some(&MachineValue::String("/usr/bin/gcc".to_string()))
        );
    }

    #[test]
    fn test_section_composition() {
        let content = r#"
[constants]
a = 'Foo'
b = a + 'World'

[constants]
a = 'Hello'
"#;
        let machine_file = MachineFile::parse(content).unwrap();

        assert_eq!(
            machine_file.get("constants", "b"),
            Some(&MachineValue::String("HelloWorld".to_string()))
        );
    }

    #[test]
    fn test_section_composition_err() {
        let content = r#"
[constants]
b = a + 'World'

[constants]
a = 'Hello'
"#;
        MachineFile::parse(content).unwrap_err();
    }

    #[test]
    fn test_section_composition_ok() {
        let content = r#"
[constants]
a = 'Hello'

[constants]
b = a + 'World'
"#;
        let machine_file = MachineFile::parse(content).unwrap();

        assert_eq!(
            machine_file.get("constants", "b"),
            Some(&MachineValue::String("HelloWorld".to_string()))
        );
    }
}
