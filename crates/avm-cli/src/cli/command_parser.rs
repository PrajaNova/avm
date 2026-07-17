fn split_command_template(input: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    enum State {
        Normal,
        SingleQuote,
        DoubleQuote,
        Escape,
        DoubleEscape,
    }
    let mut state = State::Normal;

    for ch in input.chars() {
        state = match state {
            State::Normal => match ch {
                '\'' => State::SingleQuote,
                '"' => State::DoubleQuote,
                '\\' => State::Escape,
                c if c.is_whitespace() => {
                    if !current.is_empty() {
                        tokens.push(std::mem::take(&mut current));
                    }
                    State::Normal
                }
                _ => {
                    current.push(ch);
                    State::Normal
                }
            },
            State::SingleQuote => {
                if ch == '\'' {
                    State::Normal
                } else {
                    current.push(ch);
                    State::SingleQuote
                }
            }
            State::DoubleQuote => {
                if ch == '"' {
                    State::Normal
                } else if ch == '\\' {
                    State::DoubleEscape
                } else {
                    current.push(ch);
                    State::DoubleQuote
                }
            }
            State::Escape => {
                current.push(ch);
                State::Normal
            }
            State::DoubleEscape => {
                match ch {
                    '"' | '\\' | '$' | '`' => current.push(ch),
                    _ => {
                        current.push('\\');
                        current.push(ch);
                    }
                }
                State::DoubleQuote
            }
        };
    }

    match state {
        State::Normal | State::Escape | State::DoubleEscape => {
            if !current.is_empty() {
                tokens.push(current);
            }
            Ok(tokens)
        }
        State::SingleQuote | State::DoubleQuote => Err(anyhow!("unterminated quoted command")),
    }
}
