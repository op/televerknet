//! Parser for implementing telnet clients
//!
//! [`Parser`] is implemented based on [Joe Wilm's vte library] and uses a state machine which is
//! heavily influenced by [Paul Williams' ANSI parser state machine].
//!
//! [`Parser`]: struct.Parser.html
//! [Joe Wilm's vte library]: https://github.com/jwilm/vte
//! [Paul Williams' ANSI parser state machine]: https://vt100.net/emu/dec_ansi_parser
extern crate log;

pub mod command;
pub mod option;
pub mod q;


const MAX_INTERMEDIATES: usize = 1024;
const MAX_SUBS: usize = 8;
// const MAX_PARAMS: usize = 16;

// TODO: add data to enums?
#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum State {
    // This isn't a real state.
    // Anywhere,

    // Ground is the initial state of the parser, and the state used to consume all characters
    // other than special escape characters.
    Ground,
    // Data is found and triggered from new line or GA command.
    // Data,
    Data,
    // IacEntry is entered when command IAC is recognised.
    IacEntry,
    // NegEntry is entered from IAC, for WILL, WONT, DO and DONT.
    NegEntry,
    // SubEntry is entered from IAC, for SB until SE is encountered. SubEntry is the initial state
    // for sub entries, and then the state is moved to SubIntermediate.
    SubEntry,
    // SubIntermediate is transition to from SubEntry.
    SubIntermediate,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum Action {
    None,
    Clear,
    Collect,
    Execute,
    DataDispatch,
    IacDispatch,
    NegStart,
    NegDispatch,
    SubStart,
    SubPut,
    SubDispatch,
    Ignore,
}

impl State {
    /// Get entry action for this state
    #[inline(always)]
    pub fn entry_action(&self) -> Action {
        match self {
            State::Ground => Action::None,
            State::Data => Action::DataDispatch,
            State::IacEntry => Action::DataDispatch,
            State::NegEntry => Action::None,
            State::SubEntry => Action::SubStart,
            State::SubIntermediate => Action::None,
        }
    }

    /// Get exit action for this state
    #[inline(always)]
    pub fn exit_action(&self) -> Action {
        match self {
            State::Ground => Action::None,
            State::Data => Action::Clear,
            State::IacEntry => Action::Clear,
            State::NegEntry => Action::None,
            State::SubEntry => Action::None,
            State::SubIntermediate => Action::None,
        }
    }
}

/// Parser for raw _Telnet_ protocol which delegates actions to a [`Perform`]
///
/// [`Perform`]: trait.Perform.html
pub struct Parser {
    state: State,
    intermediates: [u8; MAX_INTERMEDIATES],
    intermediate_idx: usize,
    neg_command: u8,
    subs: [u8; MAX_SUBS],
    sub_idx: usize,
    ignoring: bool,
}

impl Parser {
    pub fn new() -> Parser {
        Parser {
            state: State::Ground,
            intermediates: [0u8; MAX_INTERMEDIATES],
            intermediate_idx: 0,
            neg_command: 0,
            subs: [0u8; MAX_SUBS],
            sub_idx: 0,
            ignoring: false,
        }
    }

    #[inline]
    fn intermediates(&self) -> &[u8] {
        &self.intermediates[..self.intermediate_idx]
    }

    #[inline]
    fn subs(&self) -> &[u8] {
        &self.subs[..self.sub_idx]
    }

    /// Advance the parser state
    ///
    /// Requires a [`Perform`] in case `byte` triggers an action
    ///
    /// [`Perform`]: trait.Perform.html
    #[inline]
    pub fn advance<P: Perform>(&mut self, performer: &mut P, byte: u8) {
        let (state, action) = self.get_action(byte);
        self.perform_state_change(performer, state, action, byte);
    }

    fn get_action(&mut self, byte: u8) -> (State, Action) {
        // TODO: create lookup table for this?
        match self.state {
            State::Ground | State::Data => {
                match byte {
                    // Non-printable bytes
                    0x00..=0x1f => (State::Data, Action::Execute),
                    // Collect printable characters
                    0x20..=0x7f => (State::Ground, Action::Collect),
                    // Various commands
                    0x80..=0xfe => (State::Data, Action::Execute),
                    // Beginning of IAC sequence
                    0xff => (State::IacEntry, Action::None),
                }
            }
            State::IacEntry => {
                match byte {
                    // Beginning of subnegotation
                    0xfa => (State::SubEntry, Action::None),
                    // Beginning of negotation using WILL, WONT, DO or DONT
                    0xfb..=0xfe => (State::NegEntry, Action::NegStart),
                    // Command to dispatch to interpret
                    _ => (State::Ground, Action::IacDispatch),
                }
            }
            State::NegEntry => (State::Ground, Action::NegDispatch),
            State::SubEntry | State::SubIntermediate => {
                match byte {
                    // End of subnegotiation parameters
                    0xf0 => (State::Ground, Action::SubDispatch),
                    // Continuation of subnegotation
                    _ => (State::SubIntermediate, Action::SubPut),
                }
            }
        }
    }

    #[inline]
    fn perform_state_change<P>(&mut self, performer: &mut P, state: State, action: Action, byte: u8)
    where
        P: Perform,
    {
        macro_rules! maybe_action {
            ($action:expr, $arg:expr) => {
                match $action {
                    Action::None => (),
                    action => {
                        self.perform_action(performer, action, $arg);
                    }
                }
            };
        }

        match state {
            state => {
                // Exit action for previous state
                let exit_action = self.state.exit_action();
                maybe_action!(exit_action, 0);

                // Transition action
                maybe_action!(action, byte);

                // Entry action for new state
                maybe_action!(state.entry_action(), 0);

                // Assume the new state
                self.state = state;
            }
        }
    }

    #[inline]
    fn perform_action<P: Perform>(&mut self, performer: &mut P, action: Action, byte: u8) {
        match action {
            Action::Execute => performer.execute(byte),
            Action::Collect => {
                if self.intermediate_idx == MAX_INTERMEDIATES {
                    self.ignoring = true;
                } else {
                    self.intermediates[self.intermediate_idx] = byte;
                    self.intermediate_idx += 1;
                }
            }
            Action::DataDispatch => {
                if self.intermediate_idx > 0 {
                    performer.data(self.intermediates(), self.ignoring);
                }
            }
            Action::Ignore | Action::None => (),
            Action::Clear => {
                self.intermediate_idx = 0;
                self.ignoring = false;
            }
            Action::IacDispatch => performer.iac_dispatch(byte),
            Action::NegStart => {
                self.neg_command = byte;
            }
            Action::NegDispatch => performer.negotiate_dispatch(self.neg_command, byte),
            Action::SubStart => {
                self.sub_idx = 0;
            }
            Action::SubPut => {
                let sub_idx = self.sub_idx;
                if sub_idx < MAX_SUBS {
                    self.subs[sub_idx] = byte;
                    self.sub_idx += 1;
                }
            }
            Action::SubDispatch => {
                if self.sub_idx > 0 {
                    performer.sub_dispatch(self.subs());
                }
            }
        }
    }
}

pub trait Perform {
    /// Data event: for DATA and SEND events
    // TODO: rename to hook?
    fn data(&mut self, intermediates: &[u8], ignore: bool);

    fn execute(&mut self, byte: u8);

    /// WARNING and ERROR events
    // fn error(&mut self);

    /// Command event: for IAC
    fn iac_dispatch(&mut self, byte: u8);

    /// Command event: for IAC SUB ...
    fn sub_dispatch(&mut self, subs: &[u8]);

    /// Negotiate event: WILL, WONT, DO, DONT
    fn negotiate_dispatch(&mut self, cmd: u8, opt: u8);

    // TODO: duplicate from sub_dispathch?
    /// Subnegotiate event
    fn subnegotiate_dispatch(&mut self, params: &[u8], opt: u8);

    /// ZMP event
    fn zmp_dispatch(&mut self, params: &[&[u8]]);

    /// TTYPES event
    fn ttypes_dispatch(&mut self, cmd: u8, terminal_type: &[u8]);

    /// Compress event
    fn compress_dispatch(&mut self, state: u8);

    // TODO: environ_dispatch
    // TODO: mssp_dispatch
}

#[cfg(test)]
#[macro_use]
extern crate std;

#[cfg(test)]
extern crate env_logger;

#[cfg(test)]
mod tests {
    use super::{Parser, Perform};
    // use core::i64;
    use std::vec::Vec;

    fn init_test_logging() {
        let _ = env_logger::builder()
            .is_test(true)
            .default_format_timestamp(false)
            .filter(None, log::LevelFilter::Trace)
            .try_init();
    }

    #[derive(Default)]
    struct IacDispatcher {
        intermediates: Vec<Vec<u8>>,
        ignoring: Vec<bool>,
        execute: Vec<u8>,
        iac: Vec<u8>,
        negs: Vec<(u8, u8)>,
        subs: Vec<Vec<u8>>,
    }

    // All empty bodies except iac_dispatch
    impl Perform for IacDispatcher {
        fn data(&mut self, intermediates: &[u8], ignoring: bool) {
            self.intermediates.push(intermediates.to_vec());
            self.ignoring.push(ignoring);
        }
        fn execute(&mut self, byte: u8) {
            self.execute.push(byte);
        }
        fn iac_dispatch(&mut self, byte: u8) {
            self.iac.push(byte);
        }
        fn sub_dispatch(&mut self, subs: &[u8]) {
            self.subs.push(subs.to_vec());
        }
        fn negotiate_dispatch(&mut self, cmd: u8, opt: u8) {
            self.negs.push((cmd, opt));
        }
        fn subnegotiate_dispatch(&mut self, _params: &[u8], _opt: u8) {}
        fn zmp_dispatch(&mut self, _params: &[&[u8]]) {}
        fn ttypes_dispatch(&mut self, _cmd: u8, _terminal_type: &[u8]) {}
        fn compress_dispatch(&mut self, _state: u8) {}
    }

    #[test]
    fn parse_iac() {
        init_test_logging();

        static BYTES: &'static [u8] = &[
            255, // IAC
            246, // AYT
        ];

        let mut dispatcher = IacDispatcher::default();
        let mut parser = Parser::new();
        for byte in BYTES {
            parser.advance(&mut dispatcher, *byte);
        }

        assert_eq!(dispatcher.iac.len(), 1);
        assert_eq!(dispatcher.iac[0], 246);
    }

    #[test]
    fn parse_iac_will() {
        init_test_logging();

        static BYTES: &'static [u8] = &[
            255, // IAC
            251, // WILL
            24,  // TERMINAL-TYPE
        ];

        let mut dispatcher = IacDispatcher::default();
        let mut parser = Parser::new();
        for byte in BYTES {
            parser.advance(&mut dispatcher, *byte);
        }

        assert_eq!(dispatcher.negs.len(), 1);
        assert_eq!(dispatcher.negs[0].0, 251);
        assert_eq!(dispatcher.negs[0].1, 24);
    }

    #[test]
    fn parse_mixed_iac_will() {
        init_test_logging();

        static BYTES: &'static [u8] = &[
            b'r', b's', // data
            255,  // IAC
            251,  // WILL
            24,   // TERMINAL-TYPE
        ];

        let mut dispatcher = IacDispatcher::default();
        let mut parser = Parser::new();
        for byte in BYTES {
            parser.advance(&mut dispatcher, *byte);
        }

        assert_eq!(dispatcher.intermediates.len(), 1);
        assert_eq!(dispatcher.intermediates[0], &[b'r', b's']);
        assert_eq!(dispatcher.negs.len(), 1);
        assert_eq!(dispatcher.negs[0].0, 251);
        assert_eq!(dispatcher.negs[0].1, 24);
    }

    #[test]
    fn parse_iac_sb() {
        init_test_logging();

        static BYTES: &'static [u8] = &[
            255, // IAC
            250, // SB (start subnegotiation)
            24,  // TERMINAL-TYPE
            1,   // SEND
            255, // IAC
            240, // SA (end subnegotiation)
        ];

        let mut dispatcher = IacDispatcher::default();
        let mut parser = Parser::new();
        for byte in BYTES {
            parser.advance(&mut dispatcher, *byte);
        }

        assert_eq!(dispatcher.subs.len(), 1);
        assert_eq!(dispatcher.subs[0], &BYTES[2..(BYTES.len() - 1)]);
    }

    #[test]
    fn parse_crlf() {
        init_test_logging();

        let mut dispatcher = IacDispatcher::default();
        let mut parser = Parser::new();
        for byte in &[b'r', b's', 0x0d, 0x0a] {
            parser.advance(&mut dispatcher, *byte);
        }

        assert_eq!(dispatcher.intermediates.len(), 1);
        assert_eq!(dispatcher.intermediates[0], &[b'r', b's']);
        assert_eq!(dispatcher.execute.len(), 2);
        assert_eq!(dispatcher.execute[0], 0x0d);
        assert_eq!(dispatcher.execute[1], 0x0a);
    }

    #[test]
    fn parse_ayt() {
        init_test_logging();

        let mut dispatcher = IacDispatcher::default();
        let mut parser = Parser::new();
        for byte in &[b'r', 246, b's', 0x0d, 0x0a] {
            parser.advance(&mut dispatcher, *byte);
        }

        assert_eq!(dispatcher.execute.len(), 3);
        assert_eq!(dispatcher.execute[0], 246);
        assert_eq!(dispatcher.execute[1], 0x0d);
        assert_eq!(dispatcher.execute[2], 0x0a);
        assert_eq!(dispatcher.intermediates.len(), 2);
        assert_eq!(dispatcher.intermediates[0], &[b'r']);
        assert_eq!(dispatcher.intermediates[1], &[b's']);
    }
}
