Fuzzy
=====

Fuzzy is a tool which finds the optimal inexact match between a regex-like
pattern and a text.

I am rewriting my previous [Scala fuzzy] implementation in Rust in order to
learn the language. The code will be naive and idiosyncratic Rust to start
with, but I should make it more idiomatic over time.

The main algorithm itself still has a lot of room to support more pattern
features, as well as practical tweaks to make the match better. We'll see this
in the examples below.

What is the optimal inexact match?
----------------------------------

Fuzzy finds the minimal changes between a hypothetical text which matches a
given regex subset, to the actual given text, like this:

```
$ fuzzy -i "Helloo* World" "Helloooooo world"
Helloooooo [-W-]{+w+}orld
```

This pattern allowed multiple `o` characters in `Hello`, so fuzzy realizezs
that no changes are required in the text there. On the other hand, the text had
a lower-case `w` compared to the pattern's capital, so fuzzy reported the
change.

Fuzzy only supports a small number of regex features in the pattern at the
moment:

- literals: `abc`, `\(abc\)`
- wildcards: `.`
- character ranges: `[abc]`, `[a-zA-Z]`, `[^123]`
- zero or more repetitions: `a*`, `.(,.)*`
- nesting: `(ab*)*`, `(<([0-9]*,)*[0-9]*> )*<([0-9]*,)*[0-9]*>`

For example:

```
$ fuzzy -i bar baz
ba[-r-]{+z+}

$ fuzzy -i ... foo
foo

$ fuzzy -i '\([a-z0-9]*\)' '(1st place)'
(1st{+ +}place)

$ fuzzy -i '(<([0-9]*,)*[0-9]*> )*<([0-9]*,)*[0-9]*>' '<12,34,56> <789> <'
<12,34,56> <789> <[->-]
```

But fuzzy does not yet support other useful regex features, like alternation
or bounded repetition:

```
$ fuzzy -i 'ab|bc' ab
Error: PatternUnsupported("Alternation([Literal(\"ab\"), Literal(\"bc\")])")

$ fuzzy -i 'a+' a
Error: PatternUnsupported("Repetition(Repetition { min: 1, max: None, greedy: true, sub: Literal(\"a\") })")

$ fuzzy -i 'a{4}' a
Error: PatternUnsupported("Repetition(Repetition { min: 4, max: Some(4), greedy: true, sub: Literal(\"a\") })")
```

The underlying fuzzy algorithm keeps track of how closely the text matches the
pattern, and also records what text was captured by `()` groups, but the tool's
current output does not display this.

Practical uses of Fuzzy
-----------------------

The Fuzzy tool was originally created to help understand tens of thousands of
generated code files which had been created from different versions of
different templates with different parameters over time, and subsequently
modified manually. The goal was to discover which template each code file was
closest to, recover the original parameters, and identify what had been
changed.

Fuzzy needs more pattern features to handle this use case.

The `examples/` folder has some simpler cases. First, we'll check our `LICENSE`
against an MIT license template:

```
$ fuzzy examples/mit_pattern LICENSE
{+MIT License

+}Copyright (c) 2023 Sam Roberts

Permission is hereby granted, free of charge, to any person obtaining a copy[- -]{+
+}of this software and associated documentation files (the "Software"), to deal[- -]{+
+}in the Software without restriction, including without limitation the rights[- -]{+
+}to use, copy, modify, merge, publish, distribute, sublicense, and/or sell[- -]{+
+}copies of the Software, and to permit persons to whom the Software is[- -]{+
+}furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all[- -]{+
+}copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR[- -]{+
+}IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,[- -]{+
+}FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE[- -]{+
+}AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER[- -]{+
+}LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,[- -]{+
+}OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE[- -]{+
+}SOFTWARE.
```

Our `LICENSE` has a header that is not in the template. What's more, the
template does not split paragraphs into separate lines, so fuzzy replaces
spaces in the pattern with newlines in the text. Fuzzy would do a better job if
it had options to ignore whitespace, or tokenise the paragraphs into words
before running the diff.

Next, we'll try to use fuzzy on our `Cargo.toml` file:

```
$ fuzzy examples/cargo_pattern Cargo.toml
[package]
name = "fuzzy"
version = "0.1.0"
authors = ["Sam Roberts"]
edition = "2021"
description = "Fuzzy finds the optimal inexact match between a regex-like pattern and a text"
readme = "README.md"
[-license = ""
-]repository = "https://github.com/SamRoberts/fuzzy_rust{+"
+}license = {+"+}MIT{+"
+}publish = false # don't publish anywhere, for now{+

+}# See more keys and their definitions at[-"
homepage-] [-= "-]https://[-github-]d[-c-]o[-m/-]c.rust-lang.org/cargo/reference/manifest.html[-"-]

[dependencies]
clap = { version = "4.4.3", features = ["derive"] }
regex-syntax = "0.7.5"
thiserror = "1.0.48"
[-
-]
```

Fuzzy's optimal match has a number of issues.

First, fuzzy does not recognise the `license` field, as it comes before
repository in our pattern, but after in our cargo config. We have no way of
telling fuzzy that `Cargo.toml` entries can be in any order. Supporting
alternations would help, but ultimately fuzzy may need to move beyond regex
features to handle this nicely.

Next, Fuzzy uses the `repository` line pattern to reduce the cost of some extra
lines in my `Cargo.toml` that come after `repository` but are not in the
pattern. Fuzzy skips the `"` and newline characters that complete the
repository URL, and treats the `license`, `publish`, and following comment as
if they were part of the `repository` URL: skipping any `"` or newline
characters that would have brought the repository URL to an end. It's cheaper
for fuzzy to skip the small number of control characters in the toml file that
separate entries, rather then skipping the larger number of characters in the
unexpected entries. Fuzzy would handle this better if it could penalize or
forbid skipping syntactically important text.

Finally, my `Cargo.toml` file doesn't have the `homepage` entry the pattern
expects. However, the comment line does have a URL, and fuzzy scavenges
characters from this URL to reduce the pattern characters skipped from the
`homepage` entry value. Fuzzy might benefit from a way of specifying
dependencies: the `homepage` entry value should not be matched unless the
`homepage` key was matched. Fuzzy would also do better diff'ing on words or
lines rather then individual characters.

Overall, I believe Fuzzy still needs substantial feature development before it
can interrogate generated code. It should be more useful on boilerplate text
files, which often don't have as much structure as code or configuration.

[Scala fuzzy]: https://github.com/SamRoberts/fuzzy/
