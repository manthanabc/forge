use std::io::{self, Write};

/// A console writer that handles proper formatting by tracking cursor position
#[derive(Default)]
pub struct ConsoleWriter {
    message: Option<String>,
}

impl ConsoleWriter {
    /// Writes text followed by a newline, ensuring proper formatting
    pub fn writeln(&mut self, message: impl AsRef<str>) -> io::Result<()> {
        if self.is_new_line_required() {
            writeln!(io::stdout(), "")?;
        }
        writeln!(io::stdout(), "{}", message.as_ref())?;
        io::stdout().flush()?;
        self.message = Some(format!("{}\n", message.as_ref()));
        Ok(())
    }

    /// Writes text without a newline
    pub fn write(&mut self, message: impl AsRef<str>) -> io::Result<()> {
        write!(io::stdout(), "{}", message.as_ref())?;
        io::stdout().flush()?;
        self.message = Some(message.as_ref().to_string());
        Ok(())
    }

    /// Checks if new line is required or not.
    fn is_new_line_required(&self) -> bool {
        self.message
            .as_ref()
            .map_or(false, |message| !message.ends_with('\n'))
    }
}
