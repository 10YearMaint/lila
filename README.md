# leli

## Background

**Maybe you know this scenario**: You’re tasked with improving and maintaining a complex codebase you’ve never seen before—or at least haven’t touched in ages. As a software engineer and former founder in the medical technology field, this happens to me all the time. Trying to re-enter the project’s mindset while under “brain drain” is a constant challenge, especially when it’s a side-business tool that doesn’t yield obvious returns. Spending time on this can seem inefficient, yet it’s often the only way to move forward.

We all know there are countless software architectures and standards, each valid in its own way. Every developer, like an author, has a unique, legitimate approach for creating their “text.” If you devote enough time, you can understand their perspective—like learning someone’s native language. It’s simply human to need time for **verinnerlichen**—to internalize—another person’s thought processes.

But if time flows ever onward, I wonder if **AI** can be a surrogate for our original human insights.

Picture a specialized AI “literate” that holds the entire project in its memory. It knows exactly where each UI component was implemented, how specific logic is structured, and how to adapt it all while respecting the intent of the original developer. Through such capabilities, code gets “**lebendig konserviert**”—preserved in a living state—waiting for a future purpose. Your thoughts and insights stay alive, ready to be revisited or expanded upon whenever the need arises. Perhaps it’s time we saw software as an **endless book**, authored by countless minds, never truly finished. In other words, let us become coding literates—developers who treat code with the same creativity and respect we give to literature.

## Technical

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
rustc install.rs && ./install
```

## Usage

### Extract Literate Code into normal Source Code

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

### Auto Code Format Original Literate Code

*leli* has the functionality to auto code format literated code and insert it back into its original markdown file inplace.
For this use the following command logic:

```
./target/release/leli auto --folder example
```

### Markdown to HTML Translator

If you want to create HTML files from the markdown files, you can use the following command:

```bash
./target/release/leli translate --folder example --css src/css/style.css --mermaid src/js/mermaid.min.js
./target/release/leli translate --folder example --css src/css/style.css --disable-mermaid
```

If you don't specify a CSS file, the default CSS of src/css/style.css will be used.

### Save HTML files in a Database

If you want to save the meta data of the generated HTML files to a SQLite database, you can use the following command:

```bash
./target/release/leli save --db mydatabase.sqlite
```

or

```bash
./target/release/leli save --db mydatabase.db
./target/release/leli save
```

### Chat2CodeLiterat (beta)

```bash
leli chat --model 2 \
    --prompt "Can you understand HTML code? Can you provide me an example code? With a button? And if I click the button every time, a counter gets increased?.\nAnswer:" --quantized
```


## Development

If you develop on a macOS, please use **leli** for Windows cross-compilation using [wine](https://formulae.brew.sh/cask/wine-stable) like this:

```bash
wine windows/leli.exe extract --folder example --protocol AImM
```

Update schema.rs using

```bash
diesel migration run
```

Simply add *wine* in front of the normal command.

### Auto Code Formatter

Don't forget to code format the rust code using

```bash
cargo fmt
```
