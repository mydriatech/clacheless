/*
    Copyright 2025 MydriaTech AB

    Licensed under the Apache License 2.0 with Free world makers exception
    1.0.0 (the "License"); you may not use this file except in compliance with
    the License. You should have obtained a copy of the License with the source
    or binary distribution in file named

        LICENSE-Apache-2.0-with-FWM-Exception-1.0.0

    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.
*/

//! Library errors.

use std::error::Error;
use std::fmt;

/// Cause of error.
#[derive(Debug, PartialEq, Eq)]
pub enum ClachelessErrorKind {
    /// General failure. See message for details.
    Unspecified,
    /// Connectivity related problem. See message for details.
    Connection,
    /// The object could not be found.
    NotFound,
    /// The object is not in the expected format.
    Malformed,
}

impl ClachelessErrorKind {
    /// Create a new instance with an error message.
    pub fn error_with_msg<S: AsRef<str>>(self, msg: S) -> ClachelessError {
        ClachelessError {
            kind: self,
            msg: Some(msg.as_ref().to_string()),
        }
    }

    /// Create a new instance without an error message.
    pub fn error(self) -> ClachelessError {
        ClachelessError {
            kind: self,
            msg: None,
        }
    }
}

impl fmt::Display for ClachelessErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/** Library error.

Create a new instance via [ClachelessErrorKind].
*/
#[derive(Debug)]
pub struct ClachelessError {
    kind: ClachelessErrorKind,
    msg: Option<String>,
}

impl ClachelessError {
    /// Return the type of error.
    pub fn kind(&self) -> &ClachelessErrorKind {
        &self.kind
    }
}

impl fmt::Display for ClachelessError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(msg) = &self.msg {
            write!(f, "{} {}", self.kind, msg)
        } else {
            write!(f, "{}", self.kind)
        }
    }
}

impl AsRef<ClachelessError> for ClachelessError {
    fn as_ref(&self) -> &ClachelessError {
        self
    }
}

impl Error for ClachelessError {}
