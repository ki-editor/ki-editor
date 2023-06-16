use portable_pty::{native_pty_system, PtySize};
use std::io::{Read, Write};


pub fn run_integrated_terminal(rows: u16, cols: u16) -> Result<(), anyhow::Error> {
    crossterm::terminal::enable_raw_mode()?;
    use portable_pty::CommandBuilder;

    let mut parser = vt100::Parser::new(rows, cols, 0);

    // Use the native pty implementation for the system
    let pty_system = native_pty_system();

    // Create a new pty
    let pair = pty_system.openpty(PtySize {
        rows,
        cols,
        // Not all systems support pixel_width, pixel_height,
        // but it is good practice to set it to something
        // that matches the size of the selected font.  That
        // is more complex than can be shown here in this
        // brief example though!
        pixel_width: 1,
        pixel_height: 3,
    })?;

    // Spawn a shell into the pty
    let cmd = CommandBuilder::new("bash");
    pair.slave.spawn_command(cmd)?;

    // Assume you have a writer from the master pty
    let mut writer = pair.master.take_writer()?;

    // Assume you have a reader from the master pty
    let mut reader = pair.master.try_clone_reader()?;

    // Create a buffer to store the bytes
    let mut buf = [0; 1024];

    // Use a thread to read from the reader and print the output
    let reader_thread = std::thread::spawn(move || {
        // Read from the reader until EOF or error
        loop {
            // Read some bytes
            let n = match reader.read(&mut buf) {
                Ok(n) if n == 0 => break, // EOF
                Ok(n) => n,
                Err(e) => {
                    eprintln!("Error reading from pty: {}", e);
                    break;
                }
            };

            // Convert the bytes to a string
            let output = match std::str::from_utf8(&buf[..n]) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Invalid UTF-8 sequence: {}", e);
                    continue;
                }
            };

            // Process the bytes with the parser
            parser.process(&buf);

            let column_offset = 10;
            let row_offset = 10;
            // Get the current cursor position
            let (row, column) = parser.screen().cursor_position();

            // Apply the offset to the cursor position
            let _new_line = row + row_offset;
            let _new_col = column + column_offset;

            // Set the new cursor position
            // parser
            //     .screen()
            //     .execute(format!("\x1b[{};{}H", new_line, new_col));

            // Write the output to a file
            std::fs::write("output.txt", parser.screen().contents()).unwrap();

            // Print the output
            // print!("{}", output);
            print!("{}", output);

            // Flush the standard output
            std::io::stdout().flush().unwrap();
        }
    });

    // Use another thread to read from the standard input and write to the writer
    let writer_thread = std::thread::spawn(move || {
        // Read from the standard input until EOF or error
        loop {
            // Read some bytes
            let n = match std::io::stdin().read(&mut buf) {
                Ok(n) if n == 0 => break, // EOF
                Ok(n) => n,
                Err(e) => {
                    eprintln!("Error reading from stdin: {}", e);
                    break;
                }
            };

            // Write the bytes to the writer
            if let Err(e) = writer.write_all(&buf[..n]) {
                eprintln!("Error writing to pty: {}", e);
                break;
            }
        }
    });

    // Wait for both threads to finish
    reader_thread.join().unwrap();
    writer_thread.join().unwrap();

    Ok(())
}
