/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/// Error and related types.

/**
 * Error kinds that may occur in `nah_chat`.
 */
#[derive(Debug)]
pub enum ErrorKind {
  NetworkError,
  ModelServerError,
}

impl std::fmt::Display for ErrorKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ErrorKind::NetworkError => {
        write!(f, "Network error")
      }
      ErrorKind::ModelServerError => {
        write!(f, "Model server error")
      }
    }
  }
}

/**
 * Error type of `nah_chat`.
 */
#[derive(Debug)]
pub struct Error {
  pub kind: ErrorKind,
  pub message: Option<String>,
  pub cause: Option<Box<dyn std::error::Error>>,
}

impl std::error::Error for Error {
  fn cause(&self) -> Option<&dyn std::error::Error> {
    self.cause.as_ref().and_then(|e| Some(e.as_ref()))
  }
}

impl std::fmt::Display for Error {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}: {}",
      self.kind,
      self.message.clone().unwrap_or("None".to_string()),
    )
  }
}

pub type Result<T> = std::result::Result<T, Error>;
