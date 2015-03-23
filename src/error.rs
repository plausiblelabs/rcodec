//
// Copyright (c) 2015 Plausible Labs Cooperative, Inc.
// All rights reserved.
//
// This API is based on the design of Michael Pilquist and Paul Chiusano's
// Scala scodec library: https://github.com/scodec/scodec/
//

/// Error type for codec operations.
// TODO: Perhaps we should have separate error types for codec and byte_vector
#[derive(Debug)]
pub struct Error {
    /// The error message.
    pub description: String,

    /// The stack of context strings, with outermost context identifier at the front of the vector.
    context: Vec<String>
}

impl Error {
    /// Return a new Error with the given description.
    pub fn new(description: String) -> Error {
        Error { description: description, context: Vec::new() }
    }

    /// Return a human-readable error message that includes context, if any.
    pub fn message(&self) -> String {
        if self.context.is_empty() {
            self.description.clone()
        } else {
            // TODO: Implement a proper string joiner
            let ctx = self.context.iter().fold(String::new(), |mut a, b| {
                if !a.is_empty() {
                    a.push_str("/");
                }
                a.push_str(b.as_slice());
                a
            });
            format!("{}: {}", ctx, self.description)
        }
    }
    
    /// Return a new Error with the given context identifier pushed into the context stack.
    pub fn push_context(&self, context: &String) -> Error {
        let mut new_context = self.context.clone();
        new_context.insert(0, context.to_string());
        Error { description: self.description.clone(), context: new_context }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_error_message_should_include_context_in_the_correct_order() {
        let msg = "This is a slam poem that I wrote and I am speaking the slam poem to you right now with my mouth.";
        let expected = "outer/inner: ".to_string() + msg;
        let error = Error::new(msg.to_string()).push_context(&"inner".to_string()).push_context(&"outer".to_string());
        assert_eq!(error.message(), expected);
    }
}
