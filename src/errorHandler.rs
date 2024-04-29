// pass an error message to this function to end the program
pub fn end_program_error(message: &str) {
    println!("Oops! {message}");
    std::process::exit(0);
}