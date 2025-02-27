# <span style="display:inline-block;width:18px;height:18px;background-color:#6A4C9C;margin-left:8px;"></span> Lila

**Foreword:** Plan applicable to market-stable products (including LCE products) that have a mature (API) architecture.

1. Divide the software product into a modular platform.
2. Each module gets its own module-specific AI (navigated by *Lila*).
3. Support modules with KPIs and track them transparently in the dashboard.
4. Automated, modular regulatory releases occur after each module has tested itself (#Lila.toml).
5. When an engineer selects a new TSR (Technical System Requirements) to work on, the AI analyzes where the corresponding code section is likely to be and will function as a co-programmer.

### Important

Everything that got coded using Literate Programming in markdown files, can get assessed by the Code Literat by default.\
You can decide what the Code Literat sees from the normal Source Code Files of you Codebase.

## Background

**Maybe you know this scenario**: You’re tasked with improving and maintaining a complex codebase you’ve never seen before—or at least haven’t touched in ages. As a software engineer and former founder in the medical technology field, this happens to me all the time. Trying to re-enter the project’s mindset while under “brain drain” is a constant challenge. Spending time on this can seem inefficient, yet it’s often the only way to move forward.

We all know there are countless software architectures and standards, each valid in its own way. Every developer, like an author, has a unique, legitimate approach for creating their “text.” If you devote enough time, you can understand their perspective—like learning someone’s native language. It’s simply human to need time for **verinnerlichen**—to internalize—another person’s thought processes.

But if time flows ever onward, I wonder if **AI** can be a surrogate for our original human insights.

Picture a specialized AI “literate” that holds the entire project in its memory. It knows exactly where each UI component was implemented, how specific logic is structured, and how to adapt it all while respecting the intent of the original developer. Through such capabilities, code gets “**lebendig konserviert**”—preserved in a living state—waiting for a future purpose. Your thoughts and insights stay alive, ready to be revisited or expanded upon whenever the need arises, even if its just the maintenance of a LCE software product. Perhaps it’s time we saw software as an **endless book**, authored by countless minds, never truly finished. In other words, let us become coding literates—developers who treat code with the same creativity and respect we give to literature.


## Pre-Requirements:

1. Create a free account at https://huggingface.co to download and access their model ecosystem.
2. Create an Access Token on the HuggingFace webpage. Store it in the .env file of this lila project like this: HUGGINGFACEHUB_API_TOKEN=SecretKey


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

```bash
lila init
lila server
```


> **Model-ID:**
>
> Battle Tested SLM Models to run On-Premise:
>
> ⭐ [microsoft/Phi-3.5-mini-instruct](https://huggingface.co/microsoft/Phi-3.5-mini-instruct) \
> ⭐ [Qwen/Qwen2.5-Coder-3B-Instruct](https://huggingface.co/Qwen/Qwen2.5-Coder-3B-Instruct)
>
> Not yet tested:
>
> [deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B](https://huggingface.co/deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B) \
> [microsoft/Phi-4-mini-instruct](https://huggingface.co/microsoft/Phi-4-mini-instruct)

and in a new terminal instance:

```bash
lila prepare --folder example
lila weave --folder example --output book
```

Afterwards, if you want to chat with the book, execute in a new terminal this:

```bash
cd MiniCodeLiterat
npm install
node service.js
```

And finally this command also in a new terminal:

```bash
ng serve
```

Have fun chatting with your book chapters!

## Working seamlessly with Source Code and Literate Code

### 1. Extract Literate Code into normal Source Code

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


## Q&A

Q: What is **lila**?

A: **lila** stands for "**Li**terate **L**egacy **A**ssistant". Its designed primarily to empower the use of the *AImM* (AI-maintained Microservices) architecture.
Using **lila** is about coding with the end in mind: envisioning that your project will someday be a legacy project, which you yourself will not maintain anymore. But you want to ensure that the AI can maintain, explain, and customize it, understanding your literate words and thoughts behind it.
