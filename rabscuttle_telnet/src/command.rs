use std::convert::From;
use std::fmt;

/// A telnet command or special values.
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Command(u8);

/// A possible error value when converting a `Command` from a `u8`.
#[derive(Debug)]
pub struct InvalidCommand {
    _priv: (),
}

// #[derive(Debug)]
// pub enum ParseError {
//     InvalidCommand,
// }

// #[derive(Debug)]
// pub enum TelnetError {
//     ParseError(ParseError),
// }

// impl fmt::Display for TelnetError {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         fmt::Debug::fmt(self, f)
//     }
// }

// impl Error for TelnetError {
//     fn description(&self) -> &str {
//         match self {
//             &TelnetError::ParseError(ref e) => match *e {
//                 ParseError::InvalidCommand => "invalid command",
//             },
//         }
//     }

//     fn cause(&self) -> Option<&dyn Error> {
//         match self {
//             &TelnetError::ParseError(..) => None,
//         }
//     }
// }

impl Command {
    // TODO: return ParseError?
    pub fn from_u8(src: u8) -> Result<Command, InvalidCommand> {
        match src {
            236...255 => Ok(Command(src)),
            _ => Err(InvalidCommand::new()),
        }
    }

    pub fn as_u8(&self) -> u8 {
        (*self).into()
    }

    pub fn canonical_reason(&self) -> Option<&'static str> {
        canonical_reason(self.0)
    }
}

impl PartialEq<u8> for Command {
    fn eq(&self, other: &u8) -> bool {
        self.as_u8() == *other
    }
}

impl PartialEq<Command> for u8 {
    fn eq(&self, other: &Command) -> bool {
        *self == other.as_u8()
    }
}

impl From<Command> for u8 {
    #[inline]
    fn from(command: Command) -> u8 {
        command.0
    }
}

impl<'a> From<&'a Command> for Command {
    fn from(t: &'a Command) -> Self {
        t.clone()
    }
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {}",
            u8::from(*self),
            self.canonical_reason().unwrap_or("<unknown command>")
        )
    }
}

impl InvalidCommand {
    fn new() -> InvalidCommand {
        InvalidCommand { _priv: () }
    }
}

macro_rules! telnet_commands {
    (
        $(
            $(#[$docs:meta])*
            ($num:expr, $konst:ident, $phrase:expr);
        )+
    ) => {
        impl Command {
        $(
            $(#[$docs])*
            pub const $konst: Command = Command($num);
        )+

        }

        fn canonical_reason(num: u8) -> Option<&'static str> {
            match num {
                $(
                $num => Some($phrase),
                )+
                _ => None
            }
        }
    }
}

telnet_commands! {
    /// Interpret as command
    (255, IAC, "IAC");
    /// Indicates the demand that the other party stop performing, or confirmation that you are no
    /// longer expecting the other party to perform, the indicated option
    (254, DONT, "DON'T");
    /// Indicates the request that the other party perform, or confirmation that you are expecting
    /// the other party to perform, the indicated option.
    (253, DO, "DO");
    /// Indicates the refusal to perform, or continue performing, the indicated option.
    (252, WONT, "WON'T");
    /// Indicates the desire to begin performing, or confirmation that you are now performing, the
    /// indicated option.
    (251, WILL, "WILL");
    /// Indicates that what follows is subnegotiation of the indicated option.
    (250, SB, "SB");
    /// The GA signal.
    (249, GA, "Go ahead");
    /// The function EL.
    (248, EL, "Erase line");
    /// The function EC.
    (247, EC, "Erase character");
    /// The function AYT.
    (246, AYT, "Are You There");
    /// The function AO.
    (245, AO, "Abort operation");
    /// The function IP.
    (244, IP, "Interrupt Process");
    /// NVT character BRK.
    (243, BREAK, "Break");
    /// The data stream portion of a Synch. This should always be accompanied by a TCP Urgent
    /// notification.
    (242, DM, "Data Mark");
    /// No operation.
    (241, NOP, "NOP");
    /// End of subnegotiation parameters.
    (240, SE, "SE");

    (239, EOR, "EOR");
    (238, ABORT, "ABORT");
    (237, SUSP, "SUSP");
    (236, EOF, "EOF");
}

#[cfg(test)]
mod test {
    use super::Command;

    #[test]
    fn command_from_u8() {
        assert_eq!(Command::from_u8(255).unwrap(), Command::IAC);
        assert_eq!(Command::IAC, 255);
        Command::from_u8(235).expect_err("unexpected command");
    }
}
