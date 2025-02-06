# <span style="display:inline-block;width:18px;height:18px;background-color:#6A4C9C;margin-left:8px;"></span> lila

## Background

**Maybe you know this scenario**: You’re tasked with improving and maintaining a complex codebase you’ve never seen before—or at least haven’t touched in ages. As a software engineer and former founder in the medical technology field, this happens to me all the time. Trying to re-enter the project’s mindset while under “brain drain” is a constant challenge, especially when it’s a side-business tool that doesn’t yield obvious returns. Spending time on this can seem inefficient, yet it’s often the only way to move forward.

We all know there are countless software architectures and standards, each valid in its own way. Every developer, like an author, has a unique, legitimate approach for creating their “text.” If you devote enough time, you can understand their perspective—like learning someone’s native language. It’s simply human to need time for **verinnerlichen**—to internalize—another person’s thought processes.

But if time flows ever onward, I wonder if **AI** can be a surrogate for our original human insights.

Picture a specialized AI “literate” that holds the entire project in its memory. It knows exactly where each UI component was implemented, how specific logic is structured, and how to adapt it all while respecting the intent of the original developer. Through such capabilities, code gets “**lebendig konserviert**”—preserved in a living state—waiting for a future purpose. Your thoughts and insights stay alive, ready to be revisited or expanded upon whenever the need arises. Perhaps it’s time we saw software as an **endless book**, authored by countless minds, never truly finished. In other words, let us become coding literates—developers who treat code with the same creativity and respect we give to literature.


## Pre-Requirements:

1. Create a free account at https://huggingface.co to download and access their model ecosystem.
2. Create an Access Token on the HuggingFace webpage. Store it in the .env file of this lila project like this: HUGGINGFACEHUB_API_TOKEN=SecretKey
3. Install *Pandoc*
4. Install the Command-line tool for Rust ORM *Diesel* on your system and run
```bash
diesel migration run
```


## Installation

Compile the project using the following command:

```bash
cargo build --release
```


### Make *lila* available globally

If you are on a Unix-like system, you can use the following command:

```bash
rustc install.rs && ./install
```

or on Windows this command in the PowerShell:

```bash
rustc install_windows.rs; if ($?) { .\install_windows.exe }
```


## Usage

You just want to chat with your literate code? Use this shortcut to get started:

```bash
lila weave --folder example
lila save
lila chat \
    --prompt "I know you got provided some markdown code. Can you say whats happening there? In which file did we define some Rust math equations? Can you enhance these equations and return me the corresponding code?"
```

### 1. Extract Literate Code into normal Source Code

```bash
./target/release/lila tangle --file example/math_operations.md
```

or for a complete folder

```bash
lila tangle --folder example
```

If you code using the *AImM* protocol you should use the following command:

```bash
lila tangle --folder example --protocol AImM
```


### 2. Weave Source Code back to Literate Code

```bash
lila weave --folder example --output book
```


### 3. Edit Code Format Original Literate Code

*lila* has the functionality to edit code format literated code and insert it back into its original markdown file inplace.
For this use the following command logic:

```bash
lila edit --folder example
```

### 4. Markdown to HTML Translator

If you want to create HTML files from the markdown files, you can use the following command:

```bash
lila render --folder example --css src/css/style.css --mermaid src/js/mermaid.min.js
lila render --folder example --css src/css/style.css --disable-mermaid
```

If you don't specify a CSS file, the default CSS of src/css/style.css will be used.

### 5. Save HTML files in a Database

If you want to save the meta data of the generated HTML files to a SQLite database, you can use the following command:

```bash
lila save
```

### 6. Chat2CodeLiterat functionality

```bash
lila chat \
    --prompt "Can you understand HTML code? Can you provide me an example code? With a button? And if I click the button every time, a counter gets increased by the number 2? Can you also add some css design within the HTML code?" \
    --no-db \
    --model-id Qwen/Qwen2.5-Coder-3B-Instruct
```


#### model-id

Battle Tested SLM Models to run On-Premise：

⭐ [microsoft/Phi-3.5-mini-instruct](https://huggingface.co/microsoft/Phi-3.5-mini-instruct)
⭐ [Qwen/Qwen2.5-Coder-3B-Instruct](https://huggingface.co/Qwen/Qwen2.5-Coder-3B-Instruct)

Not yet tested:

[deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B](https://huggingface.co/deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B)


## Development Reminder

Update schema.rs using

```bash
diesel migration run
```

### Find Outdated Packages

```bash
cargo install cargo-outdated
cargo outdated
```

### Auto Code Formatter

Don't forget to code format the rust code using

```bash
cargo fmt
```


## Q&A

Q: What is **lila**?

A: **lila** stands for "**Li**terate **L**egacy **A**ssistant". Its designed primarily to empower the use of the *AImM* (AI-maintained Microservices) architecture.
Using **lila** is about coding with the end in mind: envisioning that your project will someday be a legacy project, which you yourself will not maintain anymore. But you want to ensure that the AI can maintain, explain, and customize it, understanding your literate words and thoughts behind it.

And not only that, you also want to ensure that you in a couple of months or a new developer to your project can easily find where the functionality of each of the UI screen of your app got defined. No endless searching through the codebase. Every coder normally developes in its own way and finds some convention more naturally than others. This is ok as everyone has a different concept of its software craftmanship. In fact, this is what makes us human. Using **lila**, this doesn't matter anymore because you document your UI screens using literate programming, that get intrinsically linked to the UI screens. Each new developer can easily get the starting-point for implementing or adapting something in the UI screen! This is the concept of *Locality of Behaviour*.
