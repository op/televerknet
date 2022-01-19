# Televerknet

Parser for implementing a telnet client in Rust.

The parser is implemented based on [Joe Wilm's vte library] and uses a state
machine which is heavily influenced by [Paul Williams' ANSI parser state
machine].

The negotiator is an implementation of Daniel J. Bernstein's the Q Method of
Implementing TELNET Option Negotiation as described in [RFC 1143].

[Joe Wilm's vte library]: https://github.com/jwilm/vte
[Paul Williams' ANSI parser state machine]: https://vt100.net/emu/dec_ansi_parser
[RFC 1143]: https://www.rfc-editor.org/rfc/rfc1143.html
