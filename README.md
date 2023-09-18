Fuzzy
=====

Fuzzy is a tool which finds the optimal inexact match between a regex-like
pattern and a text.

I am rewriting my previous [Scala fuzzy] implementation in Rust in order to
learn the language. The code will be naive and idiosyncratic Rust to start
with, but I should make it more idiomatic over time.

The main algorithm itself still has a lot of room to support more pattern
features, as well as practical tweaks to make the match better. For instance,
at the moment, you are likely to have `.*` somewhere in your pattern, and the
optimal match is likely to pair a lot of text with that wildcard when it could
have been paired with specific characters elsewhere in the pattern.

[Scala fuzzy]: https://github.com/SamRoberts/fuzzy/
