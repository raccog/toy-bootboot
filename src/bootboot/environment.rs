use alloc::string::String;

#[derive(Clone, Copy, Debug)]
pub enum ParseError {
    IncompleteComment,
    TooLarge,
}

/// Bootboot environment
pub struct Environment {
    pub env: String,
}

impl Environment {
    pub fn parse(raw_env: &String) -> Result<Self, ParseError> {
        let mut env = String::with_capacity(4096);
        let mut i: usize = 0;
        let mut single_comment = false;
        let mut multi_comment = false;
        let mut chars = raw_env.chars();
        let mut is_start = true;

        loop {
            // Get next char from input string
            let c = chars.next();
            // Check for end of input string
            if c.is_none() {
                // Check for incomplete comment
                if multi_comment || single_comment {
                    return Err(ParseError::IncompleteComment);
                }
                break;
            }
            let c = c.unwrap();

            // Ignore multi-line comments
            if multi_comment {
                // Check for end of multi-line comment
                if c == '*' {
                    let c = chars.next();
                    // Check for incomplete comment
                    if c.is_none() {
                        return Err(ParseError::IncompleteComment);
                    }
                    let c = c.unwrap();
                    // Check for delimiters
                    if c == '/' {
                        multi_comment = false;
                    }
                }
                continue;
            }

            // Ignore single-line comments
            if single_comment {
                // Check for new-line delimiter
                if c == '\n' {
                    single_comment = false;
                }
                continue;
            }

            // Check for single or multi-line comments
            if c == '/' {
                let c = chars.next();
                // Check for end of raw environment
                if c.is_none() {
                    env.push('/');
                    break;
                }
                let c = c.unwrap();
                // Check for delimiters
                match c {
                    '/' => {
                        single_comment = true;
                    }
                    '*' => {
                        multi_comment = true;
                    }
                    _ => {
                        // Push chars if not start of comment
                        env.push('/');
                        env.push(c);
                    }
                }
                continue;
            }

            if c == '\n' {
                // Ensure there are no newlines at the start
                // Also ensure there are not multiple newlines in a row
                if is_start || env.ends_with("\n") {
                    continue;
                }
            }

            // Push chars to environment
            env.push(c);
            if is_start {
                is_start = false;
            }
        }

        // Ensure that environment is not larger than 4KiB
        if env.as_bytes().len() > 4096 {
            return Err(ParseError::TooLarge);
        }

        Ok(Environment { env })
    }
}
