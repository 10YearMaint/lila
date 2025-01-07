# leli

What is **leli**? **leli** stands for "**le**gacy **li**terate".
Its designed primarily to empower the use of the *AImM* (AI-maintained Microservices) architecture.
**leli** prepares everything so that an AI can maintain and inspect compliant codebases by reading their HTML output files.

Using **leli** is about coding with the end in mind: envisioning that your project will someday be a legacy project, which you yourself will not maintain anymore. But you want to ensure that the AI can maintain, explain, and customize it, understanding your literate words and thoughts behind it.

And not only that, you also want to ensure that you in a couple of months or a new developer to your project can easily find where the functionality of each of the UI screen of your app got defined. No endless searching through the codebase. Every coder normally developes in its own way and finds some convention more naturally than others. This is ok as everyone has a different concept of its software craftmanship. In fact, this is what makes us human. Using **leli**, this doesn't matter anymore because you document your UI screens using literate programming, that get intrinsically linked to the UI screens. Each new developer can easily get the starting-point for implementing or adapting something in the UI screen! This is the concept of *Locality of Behaviour*.

## Installation

Compile the project using the following command:

```bash
cargo build --release
```

or if you are on a Windows machine:

```bash
cargo build --release --target x86_64-pc-windows-gnu
```

If you are on a Windows machine please also install "Diesel" using the following command:

```bash
powershell -c "irm https://github.com/diesel-rs/diesel/releases/download/v2.2.1/diesel_cli-installer.ps1 | iex"
```

And please also install "Pandoc"

### Make *leli* available globally

If you are on a Unix-like system, you can use the following command:

```bash
rustc install.rs
./install
```

## Usage

```bash
./target/release/leli extract --file example/math_operations.md
```

or for a complete folder

```bash
./target/release/leli extract --folder example
```

If you code using the AImM protocol you should use the following command:

```bash
./target/release/leli extract --folder example --protocol AImM
```

If you want to create HTML files from the markdown files, you can use the following command:

```bash
./target/release/leli translate --folder example --css src/css/style.css --mermaid src/js/mermaid.min.js

```

If you don't specify a CSS file, the default CSS of src/css/style.css will be used.

If you want to save the meta data of the generated HTML files to a SQLite database, you can use the following command:

```bash
 ./target/release/leli save --file doc/created_html_files.txt --db mydatabase.db
```

or

```bash
 ./target/release/leli save --file doc/created_html_files.txt --db mydatabase.sqlite
```


## Development

If you develop on a macOS, please use **leli** for Windows cross-compilation using [wine](https://formulae.brew.sh/cask/wine-stable) like this:

```bash
wine windows/leli.exe extract --folder example --protocol AImM
```

Simply add *wine* in front of the normal command.

### Auto Code Formatter

Don't forget to code format the rust code using

```bash
cargo fmt
```
