// This project should locate the 'weight' for each file in a project and genereate a report.
// This weight can be for files.

// Weight is dependent on lines of code, amount of for loops, if statements, and other tokens.
// line of code = 1 point
// for loop = 2 points
// if statement = 3 points
// the above is configurable via ~/.w8/config.yml

use std::fs;

fn main() {
    read_directory(".");
    println!("Hello, world!");
}

fn read_directory(path: &str) {
    let paths = fs::read_dir(path).unwrap();
    for path in paths {
        let x = path.unwrap();
        println!("{:?}", x.file_name());
        // path.unwrap().file_type().unwrap();
    }
}
