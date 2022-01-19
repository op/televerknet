//! The Q Method of Implementing TELNET Option Negotiation
//!
//! [RFC 1143]: http://www.faqs.org/rfcs/rfc1143.html
extern crate log;

use crate::command::Command;

const MAX_OPTIONS: usize = 256;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum OptionState {
    No,
    WantNo,
    WantYes,
    Yes,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum QueueBit {
    Empty,
    Opposite,
}

#[derive(Debug)]
pub enum NegotiatorError {
    AlreadyEnabled,
    AlreadyQueued,
    AlreadyDisabled,
    AlreadyNegotiating,
    DontAnsweredByWill,
    WontAnsweredByDo,
    UnknownCommand,
}

// There are two queues implemented as described by Daniel J. Bernstein in RFC 1143.
//
// If the value is true, we know that once the outstanding request is finished we will direct
// change this option again.
pub struct Negotiator {
    local: [OptionState; MAX_OPTIONS],
    localq: [QueueBit; MAX_OPTIONS],
    remote: [OptionState; MAX_OPTIONS],
    remoteq: [QueueBit; MAX_OPTIONS],
}

impl Negotiator {
    pub fn new() -> Negotiator {
        Negotiator {
            local: [OptionState::No; MAX_OPTIONS],
            localq: [QueueBit::Empty; MAX_OPTIONS],
            remote: [OptionState::No; MAX_OPTIONS],
            remoteq: [QueueBit::Empty; MAX_OPTIONS],
        }
    }

    #[inline]
    pub fn recv<P: Perform>(
        &mut self,
        performer: &mut P,
        command: Command,
        option: u8,
    ) -> Option<NegotiatorError> {
        match command {
            Command::WILL => self.recv_will(performer, option),
            Command::WONT => self.recv_wont(performer, option),
            Command::DO => self.recv_do(performer, option),
            Command::DONT => self.recv_dont(performer, option),
            _ => Some(NegotiatorError::UnknownCommand),
        }
    }

    #[inline]
    pub fn recv_will<P: Perform>(
        &mut self,
        performer: &mut P,
        option: u8,
    ) -> Option<NegotiatorError> {
        let u = usize::from(option);
        match (self.remote[u], self.remoteq[u]) {
            (OptionState::No, _) => {
                if performer.want_enabled(option) {
                    self.remote[u] = OptionState::Yes;
                    performer.send(Command::DO, option);
                } else {
                    performer.send(Command::DONT, option);
                }
                None
            }
            (OptionState::Yes, _) => None,
            (OptionState::WantNo, QueueBit::Empty) => {
                self.remote[u] = OptionState::No;
                Some(NegotiatorError::DontAnsweredByWill)
            }
            (OptionState::WantNo, QueueBit::Opposite) => {
                self.remote[u] = OptionState::Yes;
                self.remoteq[u] = QueueBit::Empty;
                Some(NegotiatorError::DontAnsweredByWill)
            }
            (OptionState::WantYes, QueueBit::Empty) => {
                self.remote[u] = OptionState::Yes;
                None
            }
            (OptionState::WantYes, QueueBit::Opposite) => {
                self.remote[u] = OptionState::WantNo;
                self.remoteq[u] = QueueBit::Empty;
                performer.send(Command::DONT, option);
                None
            }
        }
    }

    #[inline]
    fn recv_wont<P: Perform>(&mut self, performer: &mut P, option: u8) -> Option<NegotiatorError> {
        let u = usize::from(option);
        match (self.remote[u], self.remoteq[u]) {
            (OptionState::No, _) => None,
            (OptionState::Yes, _) => {
                self.remote[u] = OptionState::No;
                performer.send(Command::DONT, option);
                None
            }
            (OptionState::WantNo, QueueBit::Empty) => {
                self.remote[u] = OptionState::No;
                None
            }
            (OptionState::WantNo, QueueBit::Opposite) => {
                self.remote[u] = OptionState::WantYes;
                self.remoteq[u] = QueueBit::Empty;
                performer.send(Command::DO, option);
                None
            }
            (OptionState::WantYes, QueueBit::Empty) => {
                self.remote[u] = OptionState::No;
                None
            }
            (OptionState::WantYes, QueueBit::Opposite) => {
                self.remote[u] = OptionState::No;
                self.remoteq[u] = QueueBit::Empty;
                None
            }
        }
    }

    #[inline]
    fn recv_do<P: Perform>(&mut self, performer: &mut P, option: u8) -> Option<NegotiatorError> {
        let u = usize::from(option);
        match (self.local[u], self.localq[u]) {
            (OptionState::No, _) => {
                if performer.want_enabled(option) {
                    self.local[u] = OptionState::Yes;
                    performer.send(Command::WILL, option);
                } else {
                    performer.send(Command::WONT, option);
                }
                None
            }
            (OptionState::Yes, _) => None,
            (OptionState::WantNo, QueueBit::Empty) => {
                self.local[u] = OptionState::No;
                Some(NegotiatorError::WontAnsweredByDo)
            }
            (OptionState::WantNo, QueueBit::Opposite) => {
                self.local[u] = OptionState::Yes;
                self.localq[u] = QueueBit::Empty;
                Some(NegotiatorError::WontAnsweredByDo)
            }
            (OptionState::WantYes, QueueBit::Empty) => {
                self.local[u] = OptionState::Yes;
                None
            }
            (OptionState::WantYes, QueueBit::Opposite) => {
                self.local[u] = OptionState::WantNo;
                self.localq[u] = QueueBit::Empty;
                performer.send(Command::WONT, option);
                None
            }
        }
    }

    #[inline]
    fn recv_dont<P: Perform>(&mut self, performer: &mut P, option: u8) -> Option<NegotiatorError> {
        let u = usize::from(option);
        match (self.local[u], self.localq[u]) {
            (OptionState::No, _) => None,
            (OptionState::Yes, _) => {
                self.local[u] = OptionState::No;
                performer.send(Command::WONT, option);
                None
            }
            (OptionState::WantNo, QueueBit::Empty) => {
                self.local[u] = OptionState::No;
                None
            }
            (OptionState::WantNo, QueueBit::Opposite) => {
                self.local[u] = OptionState::WantYes;
                self.localq[u] = QueueBit::Empty;
                performer.send(Command::WILL, option);
                None
            }
            (OptionState::WantYes, QueueBit::Empty) => {
                self.local[u] = OptionState::No;
                None
            }
            (OptionState::WantYes, QueueBit::Opposite) => {
                self.local[u] = OptionState::No;
                self.localq[u] = QueueBit::Empty;
                None
            }
        }
    }

    #[inline]
    pub fn enable<P: Perform>(&mut self, performer: &mut P, option: u8) -> Option<NegotiatorError> {
        let u = usize::from(option);
        match (self.remote[u], self.remoteq[u]) {
            (OptionState::No, _) => {
                self.remote[u] = OptionState::WantYes;
                performer.send(Command::DO, option);
                None
            }
            (OptionState::Yes, _) => Some(NegotiatorError::AlreadyEnabled),
            (OptionState::WantNo, QueueBit::Empty) => {
                self.remoteq[u] = QueueBit::Opposite;
                None
            }
            (OptionState::WantNo, QueueBit::Opposite) => Some(NegotiatorError::AlreadyQueued),
            (OptionState::WantYes, QueueBit::Empty) => Some(NegotiatorError::AlreadyNegotiating),
            (OptionState::WantYes, QueueBit::Opposite) => {
                self.remoteq[u] = QueueBit::Empty;
                None
            }
        }
    }

    #[inline]
    pub fn disable<P: Perform>(
        &mut self,
        performer: &mut P,
        option: u8,
    ) -> Option<NegotiatorError> {
        let u = usize::from(option);
        match (self.remote[u], self.remoteq[u]) {
            (OptionState::No, _) => Some(NegotiatorError::AlreadyDisabled),
            (OptionState::Yes, _) => {
                self.remote[u] = OptionState::WantNo;
                performer.send(Command::DONT, option);
                None
            }
            (OptionState::WantNo, QueueBit::Empty) => Some(NegotiatorError::AlreadyNegotiating),
            (OptionState::WantNo, QueueBit::Opposite) => {
                self.remoteq[u] = QueueBit::Empty;
                None
            }
            (OptionState::WantYes, QueueBit::Empty) => {
                self.remoteq[u] = QueueBit::Opposite;
                None
            }
            (OptionState::WantYes, QueueBit::Opposite) => Some(NegotiatorError::AlreadyQueued),
        }
    }
}

pub trait Perform {
    fn send(&mut self, command: Command, option: u8);

    // called to see if we want a specific option enabled
    fn want_enabled(&mut self, option: u8) -> bool;
}

#[cfg(test)]
mod tests {
    use super::{Command, Negotiator, OptionState, Perform, QueueBit, MAX_OPTIONS};

    struct TestDispatcher {
        commands: Vec<(Command, u8)>,
        enabled: [bool; MAX_OPTIONS],
    }

    impl Default for TestDispatcher {
        fn default() -> Self {
            TestDispatcher {
                commands: Default::default(),
                enabled: [false; MAX_OPTIONS],
            }
        }
    }

    impl Perform for TestDispatcher {
        fn send(&mut self, command: Command, option: u8) {
            self.commands.push((command, option));
        }
        fn want_enabled(&mut self, option: u8) -> bool {
            self.enabled[usize::from(option)]
        }
    }

    #[test]
    fn rfc1143_ex1() {
        let mut it = Negotiator::new();
        let mut we = Negotiator::new();

        // both sides know that the option is on
        it.local[200] = OptionState::Yes;
        it.remote[200] = OptionState::Yes;
        we.local[200] = OptionState::Yes;
        we.remote[200] = OptionState::Yes;

        let mut dispatcher = TestDispatcher::default();

        // 1. it decides to disable
        it.disable(&mut dispatcher, 200);
        assert_eq!(dispatcher.commands.len(), 1);
        assert_eq!(dispatcher.commands.pop().unwrap(), (Command::DONT, 200));
        assert_eq!(it.remote[200], OptionState::WantNo);
        assert_eq!(it.remoteq[200], QueueBit::Empty);

        // 2. it decides to reenable (command is queued)
        it.enable(&mut dispatcher, 200);
        assert_eq!(dispatcher.commands.len(), 0);
        assert_eq!(it.remote[200], OptionState::WantNo);
        assert_eq!(it.remoteq[200], QueueBit::Opposite);

        // 3. we receive DONT
        we.recv(&mut dispatcher, Command::DONT, 200);
        assert_eq!(dispatcher.commands.len(), 1);
        assert_eq!(dispatcher.commands.pop().unwrap(), (Command::WONT, 200));
        assert_eq!(we.local[200], OptionState::No);

        // 4. we receive DO but disagree
        we.recv(&mut dispatcher, Command::DO, 200);
        assert_eq!(dispatcher.commands.len(), 1);
        assert_eq!(dispatcher.commands.pop().unwrap(), (Command::WONT, 200));

        // 5. it receieve WONT but automatically tries to reenable
        it.recv(&mut dispatcher, Command::WONT, 200);
        assert_eq!(it.remote[200], OptionState::WantYes);
        assert_eq!(it.remoteq[200], QueueBit::Empty);
        // 6. it pushes to reenable again
        assert_eq!(dispatcher.commands.len(), 1);
        assert_eq!(dispatcher.commands.pop().unwrap(), (Command::DO, 200));

        // 7. it receives wont and gives up
        it.recv(&mut dispatcher, Command::WONT, 200);
        assert_eq!(dispatcher.commands.len(), 0);
        assert_eq!(it.remote[200], OptionState::No);

        // for whatever reason, they decides to agree with future requests
        dispatcher.enabled[200] = true;

        // 8. we receive DO and decide to agree
        we.recv(&mut dispatcher, Command::DO, 200);
        assert_eq!(dispatcher.commands.len(), 1);
        assert_eq!(dispatcher.commands.pop().unwrap(), (Command::WILL, 200));
        assert_eq!(we.local[200], OptionState::Yes);
        assert_eq!(we.localq[200], QueueBit::Empty);
        assert_eq!(we.remote[200], OptionState::Yes);
        assert_eq!(we.remoteq[200], QueueBit::Empty);

        // 9. we decide to disable. we send WONT and disable the option
        we.disable(&mut dispatcher, 200);
        assert_eq!(dispatcher.commands.len(), 1);
        assert_eq!(dispatcher.commands.pop().unwrap(), (Command::DONT, 200));
        assert_eq!(we.remote[200], OptionState::WantNo);
        assert_eq!(we.remoteq[200], QueueBit::Empty);

        // 10. it receives WILL and agrees
        it.recv(&mut dispatcher, Command::WILL, 200);
        assert_eq!(dispatcher.commands.len(), 1);
        assert_eq!(dispatcher.commands.pop().unwrap(), (Command::DO, 200));
        assert_eq!(it.remote[200], OptionState::Yes);
        assert_eq!(it.remoteq[200], QueueBit::Empty);

        // 11. it receives WONT and agrees
        it.recv(&mut dispatcher, Command::WONT, 200);
        assert_eq!(dispatcher.commands.len(), 1);
        assert_eq!(dispatcher.commands.pop().unwrap(), (Command::DONT, 200));
        assert_eq!(it.local[200], OptionState::Yes);
        assert_eq!(it.localq[200], QueueBit::Empty);
        assert_eq!(it.remote[200], OptionState::No);
        assert_eq!(it.remoteq[200], QueueBit::Empty);

        // 12. we receives DO and agrees
        we.recv(&mut dispatcher, Command::DO, 200);
        assert_eq!(we.remote[200], OptionState::WantNo);
        assert_eq!(we.remoteq[200], QueueBit::Empty);

        // 13. we receives DONT and gives up
        we.recv(&mut dispatcher, Command::DONT, 200);
        assert_eq!(we.local[200], OptionState::No);
        assert_eq!(we.localq[200], QueueBit::Empty);
    }
}
