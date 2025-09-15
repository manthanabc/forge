mod console_writer;
mod spinner;

use anyhow::Result;
pub use console_writer::*;
pub use spinner::*;

/// Manages spinner functionality for the UI
pub struct SpinnerManager<W: Writer, S: Spinner> {
    spinner: S,
    console_writer: W,
    message: Option<String>,
}

impl<W: Writer, S: Spinner> SpinnerManager<W, S> {
    pub fn new(writer: W, spinner: S) -> Self {
        Self { spinner, console_writer: writer, message: None }
    }

    // Starts the spinner
    pub fn start(&mut self, message: Option<&str>) -> Result<()> {
        let _ = self.spinner.start(message)?;
        self.message = message.map(|m| m.to_string());
        Ok(())
    }

    /// Stop the active spinner if any
    pub fn stop(&mut self, message: Option<String>) -> Result<()> {
        let _ = self.stop_internal(message, true)?;
        Ok(())
    }

    /// Stop the active spinner if any and prints the provided content.
    fn stop_internal(&mut self, message: Option<String>, new_line: bool) -> Result<()> {
        self.spinner.stop()?;
        // Then print the message if provided
        if let Some(msg) = message {
            if new_line {
                self.console_writer.writeln(&msg)?;
            } else {
                self.console_writer.write(&msg)?;
            }
        }
        self.message = None;
        Ok(())
    }

    // Writes the console with new line.
    pub fn write_ln(&mut self, message: impl ToString) -> Result<()> {
        let is_running = self.spinner.is_running();
        let prev_message = self.message.clone();
        self.stop(Some(message.to_string()))?;
        if is_running {
            self.start(prev_message.as_deref())?;
        }

        Ok(())
    }

    // Writes the console without new line.
    pub fn write(&mut self, message: impl ToString) -> Result<()> {
        let is_running = self.spinner.is_running();
        let prev_message = self.message.clone();
        self.stop_internal(Some(message.to_string()), false)?;
        if is_running {
            self.message = prev_message;
        }
        Ok(())
    }
}
