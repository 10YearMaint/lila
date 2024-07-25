---
output_filename: "math_operations"
---

# Math Operations

This is a simple example of how to document your code using literate programming.

## Code

Now, let's implement some basic math operations for our use case. Here's a Python function that can add, subtract, multiply, and divide two numbers:

```{.python .cb-code}
def simple_math(a, b):
    print(f"Addition of {a} and {b}: {a + b}")
    print(f"Subtraction of {a} from {b}: {a - b}")
    print(f"Multiplication of {a} and {b}: {a * b}")
    print(f"Division of {a} by {b}: {a / b}")
```

Use this function by passing two numbers as arguments to perform these operations.

And now, let's implement this function in Rust as well:

```{.rust .cb-code}
fn simple_math(a: i32, b: i32) {
    println!("Addition of {} and {}: {}", a, b, a + b);
    println!("Subtraction of {} from {}: {}", a, b, a - b);
    println!("Multiplication of {} and {}: {}", a, b, a * b);
    println!("Division of {} by {}: {}", a, b, a / b);
}
```

The *leli* CLI can handle both Python and Rust code snippets located in the same markdown file.
