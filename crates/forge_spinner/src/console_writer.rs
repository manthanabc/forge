use std::io;

/// Custom writer trait for console operations
pub trait Writer {
    /// Write a string to the writer and flush
    fn write(&mut self, s: &str) -> io::Result<()>;
    /// Write a string with newline to the writer and flush
    fn writeln(&mut self, s: &str) -> io::Result<()>;
}

/// A console writer that handles proper formatting by tracking cursor position
pub struct ConsoleWriter {
    message: Option<String>,
}

impl ConsoleWriter {
    pub fn new() -> Self {
        Self { message: None }
    }

    /// Checks if new line is required or not.
    fn is_new_line_required(&self) -> bool {
        self.message
            .as_ref()
            .is_some_and(|message| !message.ends_with('\n'))
    }
}

impl Writer for ConsoleWriter {
    fn write(&mut self, s: &str) -> io::Result<()> {
        use std::io::Write;
        write!(io::stdout(), "{s}")?;
        let _ = io::stdout().flush();
        self.message = Some(s.to_string());
        Ok(())
    }

    fn writeln(&mut self, s: &str) -> io::Result<()> {
        use std::io::Write;
        if self.is_new_line_required() {
            writeln!(io::stdout())?;
        }
        writeln!(io::stdout(), "{s}")?;
        let _ = io::stdout().flush();
        self.message = Some(format!("{s}\n"));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use pretty_assertions::assert_eq;

    use super::*;

    #[derive(Default)]
    struct MockWriter {
        content: Rc<RefCell<String>>,
    }

    impl MockWriter {
        fn new() -> Self {
            Self::default()
        }

        fn content(&self) -> String {
            self.content.borrow().clone()
        }
    }

    impl Writer for MockWriter {
        fn write(&mut self, s: &str) -> io::Result<()> {
            self.content.borrow_mut().push_str(s);
            Ok(())
        }

        fn writeln(&mut self, s: &str) -> io::Result<()> {
            self.content.borrow_mut().push_str(s);
            self.content.borrow_mut().push('\n');
            Ok(())
        }
    }

    impl ConsoleWriter {
        pub fn with_message<S: Into<String>>(mut self, message: S) -> Self {
            self.message = Some(message.into());
            self
        }
    }

    #[test]
    fn test_is_new_line_required_when_no_message() {
        let fixture = ConsoleWriter::new();
        let actual = fixture.is_new_line_required();
        let expected = false;
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_is_new_line_required_when_message_ends_with_newline() {
        let fixture = ConsoleWriter::new().with_message("hello\n");
        let actual = fixture.is_new_line_required();
        let expected = false;
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_is_new_line_required_when_message_does_not_end_with_newline() {
        let fixture = ConsoleWriter::new().with_message("hello");
        let actual = fixture.is_new_line_required();
        let expected = true;
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_write_outputs_correct_content() {
        let mut fixture = MockWriter::new();
        fixture.write("test message").unwrap();
        let actual = fixture.content();
        let expected = "test message".to_string();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_writeln_outputs_correct_content() {
        let mut fixture = MockWriter::new();
        fixture.writeln("test message").unwrap();
        let actual = fixture.content();
        let expected = "test message\n".to_string();
        assert_eq!(actual, expected);
    }
}
