use assert_cmd::Command;
use tempfile::NamedTempFile;
use std::io::{self, Write};

#[test]
#[should_panic] // TODO support empty patterns in the regex parser
fn match_empty() {
    let mut cmd = Command::cargo_bin("fuzzy_cli").unwrap();

    cmd
        .arg("-i")
        .arg("")
        .arg("")
        .assert()
        .stdout("\n")
        .success();
}

#[test]
fn readme_hello_world() {
    let mut cmd = Command::cargo_bin("fuzzy_cli").unwrap();

    cmd
        .arg("-i")
        .arg("Helloo* World")
        .arg("Helloooooo world")
        .assert()
        .stdout("Helloooooo [-W-]{+w+}orld\n")
        .success();
}

#[test]
fn readme_bar_baz() {
    let mut cmd = Command::cargo_bin("fuzzy_cli").unwrap();

    cmd
        .arg("-i")
        .arg("bar")
        .arg("baz")
        .assert()
        .stdout("ba[-r-]{+z+}\n")
        .success();
}

#[test]
fn readme_foo() {
    let mut cmd = Command::cargo_bin("fuzzy_cli").unwrap();

    cmd
        .arg("-i")
        .arg("...")
        .arg("foo")
        .assert()
        .stdout("foo\n")
        .success();
}

#[test]
fn readme_1st_place() {
    let mut cmd = Command::cargo_bin("fuzzy_cli").unwrap();

    cmd
        .arg("-i")
        .arg(r"\([a-z0-9]*\)")
        .arg("(1st place)")
        .assert()
        .stdout("(1st{+ +}place)\n")
        .success();
}

#[test]
fn readme_nested() {
    let mut cmd = Command::cargo_bin("fuzzy_cli").unwrap();

    cmd
        .arg("-i")
        .arg("(<([0-9]*,)*[0-9]*> )*<([0-9]*,)*[0-9]*>")
        .arg("<12,34,56> <789> <")
        .assert()
        .stdout("<12,34,56> <789> <[->-]\n")
        .success();
}

#[test]
fn readme_andrew() {
    let mut cmd = Command::cargo_bin("fuzzy_cli").unwrap();

    cmd
        .arg("-i")
        .arg(r"((title: [a-zA-Z ]*|salary: \$[0-9]*|name: [a-zA-Z ]*), )*")
        .arg("name: Andrew Ant, salary: 100,000")
        .assert()
        .stdout("name: Andrew Ant, salary: [-$-]100{+,+}000[-, -]\n")
        .success();
}

#[test]
fn readme_john() {
    let mut cmd = Command::cargo_bin("fuzzy_cli").unwrap();

    cmd
        .arg("-i")
        .arg("[a-zA-Z]+ [a-zA-Z]+")
        .arg("John Smith")
        .assert()
        .stdout("John Smith\n")
        .success();
}

#[test]
fn readme_year() {
    let mut cmd = Command::cargo_bin("fuzzy_cli").unwrap();

    cmd
        .arg("-i")
        .arg("[0-9]{4}")
        .arg("'69")
        .assert()
        .stdout("[-??-]{+'+}69\n")
        .success();
}

#[test]
fn smoke_readme_license() -> Result<(), io::Error>{
    let mut pattern = NamedTempFile::new()?;
    write!(pattern, r#"Copyright \(c\) [0-9][0-9][0-9][0-9] [^\n]*

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files \(the "Software"\), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE."#)?;


    let mut text = NamedTempFile::new()?;
    write!(text, r#"MIT License

Copyright (c) 2023 Sam Roberts

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE."#)?;

    let mut cmd = Command::cargo_bin("fuzzy_cli").unwrap();

    cmd
        .arg(pattern.path())
        .arg(text.path())
        .assert()
        .success();

    return Ok(());
}

#[test]
fn smoke_readme_cargo() -> Result<(), io::Error>{
    let mut pattern = NamedTempFile::new()?;
    write!(pattern, r#"\[package\]
name = "[^\n\"]*"
version = "[^\n\"]*"
authors = \[("[^\n\"].*", )*"[^\n\"].*"\]
edition = "[0-9][0-9][0-9][0-9]"
description = "[^\n\"]*"
readme = "README.md"
license = "[^\n\"]*"
repository = "https://github.com/[^\n\"/]*/[^\n\"]*"
homepage = "https://github.com/[^\n\"/]*/[^\n\"]*"

\[dependencies\]
([^\n]* = [^\n]*
)*"#)?;


    let mut text = NamedTempFile::new()?;
    write!(text, r#"[workspace]
members = ["fuzzy", "fuzzy_cli", "fuzzy_lambda"]

[workspace.package]
authors = ["Sam Roberts"]
edition = "2021"
readme = "README.md"
repository = "https://github.com/SamRoberts/fuzzy_rust"
license = "MIT"
license-file = "LICENSE"
publish = false # don't publish anywhere, for now

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[workspace.dependencies]
fuzzy = {{ path = "fuzzy" }}

nonempty = "0.8.1"
regex-syntax = "0.7.5"
thiserror = "1.0.48"

clap = {{ version = "4.4.3", features = ["derive"] }}

serde = {{ version = "1.0.189", features = ["derive"] }}
serde_json = "1.0.107"

# these dependencies were auto-generated by cargo lambda
lambda_http = "0.8.1"
lambda_runtime = "0.8.1"
tokio = {{ version = "1", features = ["macros"] }}
tracing = {{ version = "0.1", features = ["log"] }}
tracing-subscriber = {{ version = "0.3", default-features = false, features = ["fmt"] }}

test-case = "3.2.1"
assert_cmd = "2.0.12"
tempfile = "3.8.1""#)?;

    let mut cmd = Command::cargo_bin("fuzzy_cli").unwrap();

    cmd
        .arg(pattern.path())
        .arg(text.path())
        .assert()
        .success();

    return Ok(());
}
