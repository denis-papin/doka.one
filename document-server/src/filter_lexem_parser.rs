use std::str::Chars;
use unicode_segmentation::UnicodeSegmentation;
use crate::filter_lexem_parser::StreamMode::{Free, PendingAttribute, PendingOperator, PendingValue};
use crate::filter_token_parser::ComparisonOperator::{EQ, GT, GTE, LIKE, LT, LTE, NEQ};
use crate::filter_token_parser::{LogicalOperator, Token};
use crate::filter_token_parser::Token::BinaryLogicalOperator;

enum StreamMode {
    Free,
    PendingAttribute,
    PendingOperator,
    PendingValue,
}

#[derive(Debug)]
enum StreamMode2 {
    Free,
    PendingAttribute,
    PendingOperator,
    PendingValue,
    UnknownOpen,
    PendingLogicalOperator,
}

enum ParenthesisMode {
    RIGHT_AFTER_OPENING,
    RIGHT_AFTER_CLOSING,
    OTHER,
}

const LOP_AND : &str = "AND";
const LOP_OR : &str = "OR";
const FOP_EQ: &str = "EQ";


fn lex2(input: &str) -> Vec<Token> {
    let mut stream_mode: StreamMode = StreamMode::Free;
    let mut attribute: String = String::new();
    let mut fop: String = String::new();
    let mut lop: String = String::new();
    let mut value: String = String::new();
    let mut tokens: Vec<Token> = vec![];
    let mut parenthesis_mode: ParenthesisMode = ParenthesisMode::OTHER;

    let closed_input = format!("({})", input); // Encapsulate the conditions in a root ()

    for g in UnicodeSegmentation::graphemes(closed_input.as_str(), true) {
        let token = match  g.chars().next() {
            Some('(') => {
                match parenthesis_mode {
                    ParenthesisMode::RIGHT_AFTER_OPENING => {
                        // Corrige le token précedent
                        if let Some(dernier) = tokens.last_mut() {
                            *dernier = Token::LogicalOpen;
                        }
                    }
                    ParenthesisMode::RIGHT_AFTER_CLOSING => {

                    }
                    ParenthesisMode::OTHER => {
                    }
                }

                match stream_mode {
                    Free => {
                        match lop.as_str() {
                            LOP_AND => {tokens.push(BinaryLogicalOperator(LogicalOperator::AND))}
                            LOP_OR => {tokens.push(BinaryLogicalOperator(LogicalOperator::OR))}
                            _ => {}
                        }
                        lop.clear();
                    }
                    _ => {}
                }
                parenthesis_mode = ParenthesisMode::RIGHT_AFTER_OPENING;
                stream_mode = PendingAttribute;
                attribute.clear();
                Token::ConditionOpen
            }
            Some(')') => {
                match parenthesis_mode {
                    ParenthesisMode::RIGHT_AFTER_OPENING => {
                        panic!("Closing parenthesis cannot be here after an opening");
                    }
                    ParenthesisMode::RIGHT_AFTER_CLOSING => {
                        Token::LogicalClose
                    }
                    ParenthesisMode::OTHER => {
                        parenthesis_mode = ParenthesisMode::RIGHT_AFTER_CLOSING;
                        stream_mode = Free;
                        Token::ConditionClose
                    }
                }
            }
            Some(' ') => match stream_mode {
                Free => Token::Ignore,
                PendingAttribute => {
                    match attribute.is_empty() {
                        false => {
                            stream_mode = PendingOperator;
                            fop.clear();
                            Token::Attribute(attribute.clone())
                        }
                        true => {
                            Token::Ignore
                        }
                    }
                }
                PendingOperator => {
                    stream_mode = StreamMode::PendingValue;
                    value.clear();
                    match fop.as_str() {
                        "==" => Token::Operator(EQ),
                        "!=" => Token::Operator(NEQ),
                        ">" => Token::Operator(GT),
                        "=>" | ">=" => Token::Operator(GTE),
                        "<" => Token::Operator(LT),
                        "<=" | "=<" => Token::Operator(LTE),
                        "LIKE" => Token::Operator(LIKE),
                        _ => Token::Ignore, // TODO handle errors
                    }
                }
                PendingValue => {
                    stream_mode = Free;
                    let c_value = value.clone();
                    if value.starts_with("\"") {
                        dbg!(&c_value);
                        Token::ValueString(c_value.trim_matches('"').to_string())
                    } else {
                        match c_value.parse() {
                            Ok(parsed) => Token::ValueInt(parsed),
                            Err(_) => {
                                // TODO handle errors
                                panic!("not a number")
                            },
                        }
                    }

                }
            },
            Some(c) => {
                match stream_mode {
                    Free => {
                        println!("LOP C : {}", c);
                        lop.push(c);
                    }
                    PendingAttribute => attribute.push(c),
                    PendingOperator => fop.push(c),
                    PendingValue => value.push(c),
                }
                parenthesis_mode = ParenthesisMode::OTHER;
                Token::Ignore
            }
            None => Token::Ignore,
        };

        if token != Token::Ignore {
            tokens.push(token);
        }
    }

    tokens
}


// fn lex(input: &str) -> Vec<Token> {
//     let mut stream_mode: StreamMode = StreamMode::Free;
//     let mut attribute: String = String::new();
//     let mut fop: String = String::new();
//     let mut lop: String = String::new();
//     let mut value: String = String::new();
//     let mut tokens: Vec<Token> = vec![];
//
//     for g in UnicodeSegmentation::graphemes(input, true) {
//         let token = match  g.chars().next() {
//             Some('{') => Token::LogicalOpen,
//             Some('}') => Token::LogicalClose,
//             Some('(') => {
//                 match stream_mode {
//                     Free => {
//                         match lop.as_str() {
//                             LOP_AND => {tokens.push(BinaryLogicalOperator(LogicalOperator::AND))}
//                             LOP_OR => {tokens.push(BinaryLogicalOperator(LogicalOperator::OR))}
//                             _ => {}
//                         }
//                         lop.clear();
//                     }
//                     _ => {}
//                 }
//
//                 stream_mode = PendingAttribute;
//                 attribute.clear();
//                 Token::ConditionOpen
//             }
//             Some(')') => {
//                 stream_mode = Free;
//                 Token::ConditionClose
//             }
//             Some(' ') => match stream_mode {
//                 Free => Token::Ignore,
//                 PendingAttribute => {
//                     match attribute.is_empty() {
//                         false => {
//                             stream_mode = PendingOperator;
//                             fop.clear();
//                             Token::Attribute(attribute.clone())
//                         }
//                         true => {
//                             Token::Ignore
//                         }
//                     }
//                 }
//                 PendingOperator => {
//                     stream_mode = StreamMode::PendingValue;
//                     value.clear();
//                     match fop.as_str() {
//                         "EQ" => Token::Operator(EQ),
//                         "NEQ" => Token::Operator(NEQ),
//                         "GT" => Token::Operator(GT),
//                         "GTE" => Token::Operator(GTE),
//                         "LT" => Token::Operator(LT),
//                         "LTE" => Token::Operator(LTE),
//                         "LIKE" => Token::Operator(LIKE),
//                         _ => Token::Ignore, // TODO handle errors
//                     }
//                 }
//                 PendingValue => {
//                     stream_mode = Free;
//                     let c_value = value.clone();
//                     if value.starts_with("\"") {
//                         dbg!(&c_value);
//                         Token::ValueString(c_value.trim_matches('"').to_string())
//                     } else {
//                         match c_value.parse() {
//                             Ok(parsed) => Token::ValueInt(parsed),
//                             Err(_) => {
//                                 // TODO handle errors
//                                 panic!("not a number")
//                             },
//                         }
//                     }
//
//                 }
//             },
//             Some(c) => {
//                 match stream_mode {
//                     Free => {
//                         println!("LOP C : {}", c);
//                         lop.push(c);
//                     }
//                     PendingAttribute => attribute.push(c),
//                     PendingOperator => fop.push(c),
//                     PendingValue => value.push(c),
//                 }
//                 Token::Ignore
//             }
//             None => Token::Ignore,
//         };
//
//         if token != Token::Ignore {
//             tokens.push(token);
//         }
//     }
//
//     tokens
// }




#[cfg(test)]
mod tests {
    //cargo test --color=always --bin document-server expression_filter_parser::tests   -- --show-output

    use crate::filter_lexem_parser::{lex2};
    use crate::filter_token_parser::{parse_expression, to_canonical_form, to_sql_form};

    // /// OK
    // #[test]
    // pub fn parse_token_1() {
    //     let input = "( attribut2 EQ \"你好\" )";
    //     let tokens = lex(input);
    //     for token in &tokens {
    //         println!("TOKEN : {:?}", token);
    //     }
    //     let exp= parse_expression(&tokens).unwrap();
    //     let s = to_canonical_form(&exp).unwrap();
    //     println!("{:?}", s);
    // }
    //
    // /// OK
    // #[test]
    // pub fn parse_token_3() {
    //     let input = "{{(attribut1 GT 10 ) AND ( attribut2 EQ \"你好\" )} OR ( attribut3 LIKE \"den%\" )}";
    //     let tokens = lex(input);
    //     for token in &tokens {
    //         println!("TOKEN : {:?}", token);
    //     }
    //     let exp= parse_expression(&tokens).unwrap();
    //     let s = to_canonical_form(&exp).unwrap();
    //     println!("{:?}", s);
    // }
    //
    // /// Many "AND"
    // #[test]
    // pub fn parse_token_4() {
    //     let input = "{ (attribut1 GT 10 ) AND ( attribut2 EQ \"你好\" ) AND ( attribut3 LIKE \"den%\" )}";
    //     let tokens = lex(input);
    //     for token in &tokens {
    //         println!("TOKEN : {:?}", token);
    //     }
    //     let exp= parse_expression(&tokens).unwrap();
    //     let s = to_canonical_form(&exp).unwrap();
    //     println!("{:?}", s);
    // }

    ///
    #[test]
    pub fn parse_token_new_form_1() {
        let input = "(attribut1 => 10 ) AND (( attribut2 == \"你好\" ) OR ( attribut3 LIKE \"den%\" ))";
        let tokens = lex2(input);
        for token in &tokens {
            println!("TOKEN : {:?}", token);
        }
        let exp= parse_expression(&tokens).unwrap();
        let s = to_canonical_form(&exp).unwrap();
        println!("{:?}", s);
    }

    /// Grouping with "()"
    #[test]
    pub fn parse_token_new_form_2() {
        let input = "((attribut1 =< 10 ) AND ( attribut2 != \"你好\" )) OR ( attribut3 LIKE \"den%\" )";
        let tokens = lex2(input);
        for token in &tokens {
            println!("TOKEN : {:?}", token);
        }
        let exp = parse_expression(&tokens).unwrap();
        let s = to_canonical_form(&exp).unwrap();
        println!("{:?}", s);
    }

    /// triple conditions
    #[test]
    pub fn parse_token_fail_1() {
        let input = "(attribut1 > 10) AND ( attribut2 == \"你好\" ) OR ( attribut3 LIKE \"den%\" )";
        let tokens = lex2(input);
        for token in &tokens {
            println!("TOKEN : {:?}", token);
        }
        let exp = parse_expression(&tokens).unwrap();
        let s = to_canonical_form(&exp).unwrap();
        println!("{:?}", s);
    }

    #[test]
    pub fn parse_token_fail_2() {
        let input = "attribut1 <= 10)";
        let tokens = lex2(input);
        for token in &tokens {
            println!("TOKEN : {:?}", token);
        }
        let exp = parse_expression(&tokens).unwrap();
        let s = to_canonical_form(&exp).unwrap();
        println!("{:?}", s);
    }

}
