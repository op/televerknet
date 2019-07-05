use std::convert::From;
use std::error::Error;
use std::fmt;

/// A telnet option value.
#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct Opt(u8);

/// A possible error value when converting a `Option` from a `u8`.
#[derive(Debug)]
pub struct InvalidOption {
    invalid_src: u8,
}

impl fmt::Display for InvalidOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid option")
    }
}

impl Error for InvalidOption {
    fn description(&self) -> &str {
        "invalid option"
    }
}

impl Opt {
    // TODO: return ParseError?
    pub fn from_u8(src: u8) -> Result<Opt, InvalidOption> {
        match src {
            1...39 => Ok(Opt(src)),
            70 | 85 | 86 | 93 | 255 => Ok(Opt(src)),
            _ => Err(InvalidOption { invalid_src: src }),
        }
    }

    pub fn as_u8(&self) -> u8 {
        (*self).into()
    }

    pub fn canonical_reason(&self) -> Option<&'static str> {
        canonical_reason(self.0)
    }
}

impl PartialEq<u8> for Opt {
    fn eq(&self, other: &u8) -> bool {
        self.as_u8() == *other
    }
}

impl PartialEq<Opt> for u8 {
    fn eq(&self, other: &Opt) -> bool {
        *self == other.as_u8()
    }
}

impl From<Opt> for u8 {
    #[inline]
    fn from(option: Opt) -> u8 {
        option.0
    }
}

impl<'a> From<&'a Opt> for Opt {
    fn from(t: &'a Opt) -> Self {
        t.clone()
    }
}

impl fmt::Debug for Opt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for Opt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {}",
            u8::from(*self),
            self.canonical_reason().unwrap_or("<unknown option>")
        )
    }
}

macro_rules! telnet_options {
    (
        $(
            $(#[$docs:meta])*
            ($num:expr, $konst:ident, $phrase:expr);
        )+
    ) => {
        impl Opt {
        $(
            $(#[$docs])*
            pub const $konst: Opt = Opt($num);
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

telnet_options! {
    (0, BINARY, "BINARY");
    (1, ECHO, "ECHO");
    (2, RCP, "RCP");
    (3, SGA, "SGA");
    (4, NAMS, "NAMS");
    (5, STATUS, "STATUS");
    (6, TM, "TM");
    (7, RCTE, "RCTE");
    (8, NAOL, "NAOL");
    (9, NAOP, "NAOP");
    (10, NAOCRD, "NAOCRD");
    (11, NAOHTS, "NAOHTS");
    (12, NAOHTD, "NAOHTD");
    (13, NAOFFD, "NAOFFD");
    (14, NAOVTS, "NAOVTS");
    (15, NAOVTD, "NAOVTD");
    (16, NAOLFD, "NAOLFD");
    (17, XASCII, "XASCII");
    (18, LOGOUT, "LOGOUT");
    (19, BM, "BM");
    (20, DET, "DET");
    (21, SUPDUP, "SUPDUP");
    (22, SUPDUPOUTPUT, "SUPDUPOUTPUT");
    (23, SNDLOC, "SNDLOC");
    (24, TTYPE, "TTYPE");
    (25, EOR, "EOR");
    (26, TUID, "TUID");
    (27, OUTMRK, "OUTMRK");
    (28, TTYLOC, "TTYLOC");
    (29, _3270REGIME, "3270REGIME");
    (30, X3PAD, "X3PAD");
    (31, NAWS, "NAWS");
    (32, TSPEED, "TSPEED");
    (33, LFLOW, "LFLOW");
    (34, LINEMODE, "LINEMODE");
    (35, XDISPLOC, "XDISPLOC");
    (36, ENVIRON, "ENVIRON");
    (37, AUTHENTICATION, "AUTHENTICATION");
    (38, ENCRYPT, "ENCRYPT");
    (39, NEW_ENVIRON, "NEW_ENVIRON");
    (70, MSSP, "MSSP");
    (85, COMPRESS, "COMPRESS");
    /// Also known as MCCP 2
    /// https://tintin.sourceforge.io/protocols/mccp/
    (86, COMPRESS2, "COMPRESS2");
    (93, ZMP, "ZMP");
    (255, EXOPL, "EXOPL");
}

#[cfg(test)]
mod test {
    use super::Opt;

    #[test]
    fn option_from_u8() {
        assert_eq!(Opt::from_u8(86).unwrap(), Opt::COMPRESS2);
        assert_eq!(Opt::COMPRESS2, 86);
        // assert_eq!(Opt::MCCP2, 86);
        assert_eq!(Opt::from_u8(254).unwrap_err().invalid_src, 254);
    }
}
