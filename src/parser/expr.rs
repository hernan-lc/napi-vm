use super::{Expr, ExprOrBlock, ObjectProp, Parser, Statement};
use crate::lexer::Token;

impl Parser {
    pub(crate) fn expr(&mut self) -> Option<Expr> {
        self.comma()
    }

    /// The comma operator sits at the lowest precedence. It is only parsed in
    /// contexts that explicitly call `expr()`; list contexts (array elements,
    /// call arguments, object values, ...) call `assign()` so the comma is
    /// treated as a separator instead.
    fn comma(&mut self) -> Option<Expr> {
        let mut l = self.assign()?;
        while self.eat(&Token::Comma) {
            let r = self.assign()?;
            l = Expr::Binary {
                op: ",".to_string(),
                left: Box::new(l),
                right: Box::new(r),
            };
        }
        Some(l)
    }

    pub(crate) fn assign(&mut self) -> Option<Expr> {
        let l = self.cond()?;
        match self.cur() {
            Token::Equal => {
                self.adv();
                let v = self.assign()?;
                Some(Expr::Assignment {
                    target: Box::new(l),
                    op: "=".to_string(),
                    value: Box::new(v),
                })
            }
            Token::PlusEqual => {
                self.adv();
                let v = self.assign()?;
                Some(Expr::Assignment {
                    target: Box::new(l),
                    op: "+=".to_string(),
                    value: Box::new(v),
                })
            }
            Token::MinusEqual => {
                self.adv();
                let v = self.assign()?;
                Some(Expr::Assignment {
                    target: Box::new(l),
                    op: "-=".to_string(),
                    value: Box::new(v),
                })
            }
            Token::StarEqual => {
                self.adv();
                let v = self.assign()?;
                Some(Expr::Assignment {
                    target: Box::new(l),
                    op: "*=".to_string(),
                    value: Box::new(v),
                })
            }
            Token::SlashEqual => {
                self.adv();
                let v = self.assign()?;
                Some(Expr::Assignment {
                    target: Box::new(l),
                    op: "/=".to_string(),
                    value: Box::new(v),
                })
            }
            Token::PercentEqual => {
                self.adv();
                let v = self.assign()?;
                Some(Expr::Assignment {
                    target: Box::new(l),
                    op: "%=".to_string(),
                    value: Box::new(v),
                })
            }
            Token::StarStarEqual => {
                self.adv();
                let v = self.assign()?;
                Some(Expr::Assignment {
                    target: Box::new(l),
                    op: "**=".to_string(),
                    value: Box::new(v),
                })
            }
            Token::AmpEqual => {
                self.adv();
                let v = self.assign()?;
                Some(Expr::Assignment {
                    target: Box::new(l),
                    op: "&=".to_string(),
                    value: Box::new(v),
                })
            }
            Token::PipeEqual => {
                self.adv();
                let v = self.assign()?;
                Some(Expr::Assignment {
                    target: Box::new(l),
                    op: "|=".to_string(),
                    value: Box::new(v),
                })
            }
            Token::CaretEqual => {
                self.adv();
                let v = self.assign()?;
                Some(Expr::Assignment {
                    target: Box::new(l),
                    op: "^=".to_string(),
                    value: Box::new(v),
                })
            }
            Token::ShlEqual => {
                self.adv();
                let v = self.assign()?;
                Some(Expr::Assignment {
                    target: Box::new(l),
                    op: "<<=".to_string(),
                    value: Box::new(v),
                })
            }
            Token::ShrEqual => {
                self.adv();
                let v = self.assign()?;
                Some(Expr::Assignment {
                    target: Box::new(l),
                    op: ">>=".to_string(),
                    value: Box::new(v),
                })
            }
            Token::UShrEqual => {
                self.adv();
                let v = self.assign()?;
                Some(Expr::Assignment {
                    target: Box::new(l),
                    op: ">>>=".to_string(),
                    value: Box::new(v),
                })
            }
            _ => Some(l),
        }
    }

    fn cond(&mut self) -> Option<Expr> {
        let t = self.nullish()?;
        if self.eat(&Token::Question) {
            let c = self.assign()?;
            self.eat(&Token::Colon);
            let a = self.assign()?;
            Some(Expr::Conditional {
                test: Box::new(t),
                consequent: Box::new(c),
                alternate: Box::new(a),
            })
        } else {
            Some(t)
        }
    }

    fn nullish(&mut self) -> Option<Expr> {
        let mut l = self.or()?;
        while self.eat(&Token::QuestionQuestion) {
            let r = self.or()?;
            l = Expr::Binary {
                op: "??".to_string(),
                left: Box::new(l),
                right: Box::new(r),
            };
        }
        Some(l)
    }

    fn or(&mut self) -> Option<Expr> {
        let mut l = self.and()?;
        while self.eat(&Token::Or) {
            let r = self.and()?;
            l = Expr::Binary {
                op: "||".to_string(),
                left: Box::new(l),
                right: Box::new(r),
            };
        }
        Some(l)
    }

    fn and(&mut self) -> Option<Expr> {
        let mut l = self.bitor()?;
        while self.eat(&Token::And) {
            let r = self.bitor()?;
            l = Expr::Binary {
                op: "&&".to_string(),
                left: Box::new(l),
                right: Box::new(r),
            };
        }
        Some(l)
    }

    fn bitor(&mut self) -> Option<Expr> {
        let mut l = self.bitxor()?;
        while self.eat(&Token::BitOr) {
            let r = self.bitxor()?;
            l = Expr::Binary {
                op: "|".to_string(),
                left: Box::new(l),
                right: Box::new(r),
            };
        }
        Some(l)
    }

    fn bitxor(&mut self) -> Option<Expr> {
        let mut l = self.bitand()?;
        while self.eat(&Token::BitXor) {
            let r = self.bitand()?;
            l = Expr::Binary {
                op: "^".to_string(),
                left: Box::new(l),
                right: Box::new(r),
            };
        }
        Some(l)
    }

    fn bitand(&mut self) -> Option<Expr> {
        let mut l = self.eq()?;
        while self.eat(&Token::BitAnd) {
            let r = self.eq()?;
            l = Expr::Binary {
                op: "&".to_string(),
                left: Box::new(l),
                right: Box::new(r),
            };
        }
        Some(l)
    }

    fn eq(&mut self) -> Option<Expr> {
        let mut l = self.cmp()?;
        loop {
            match self.cur() {
                Token::EqualEqual => {
                    self.adv();
                    let r = self.cmp()?;
                    l = Expr::Binary {
                        op: "==".to_string(),
                        left: Box::new(l),
                        right: Box::new(r),
                    };
                }
                Token::NotEqual => {
                    self.adv();
                    let r = self.cmp()?;
                    l = Expr::Binary {
                        op: "!=".to_string(),
                        left: Box::new(l),
                        right: Box::new(r),
                    };
                }
                Token::EqualEqualEqual => {
                    self.adv();
                    let r = self.cmp()?;
                    l = Expr::Binary {
                        op: "===".to_string(),
                        left: Box::new(l),
                        right: Box::new(r),
                    };
                }
                Token::NotEqualEqual => {
                    self.adv();
                    let r = self.cmp()?;
                    l = Expr::Binary {
                        op: "!==".to_string(),
                        left: Box::new(l),
                        right: Box::new(r),
                    };
                }
                _ => break,
            }
        }
        Some(l)
    }

    fn cmp(&mut self) -> Option<Expr> {
        let mut l = self.shift()?;
        loop {
            match self.cur() {
                Token::Less => {
                    self.adv();
                    let r = self.shift()?;
                    l = Expr::Binary {
                        op: "<".to_string(),
                        left: Box::new(l),
                        right: Box::new(r),
                    };
                }
                Token::Greater => {
                    self.adv();
                    let r = self.shift()?;
                    l = Expr::Binary {
                        op: ">".to_string(),
                        left: Box::new(l),
                        right: Box::new(r),
                    };
                }
                Token::LessEqual => {
                    self.adv();
                    let r = self.shift()?;
                    l = Expr::Binary {
                        op: "<=".to_string(),
                        left: Box::new(l),
                        right: Box::new(r),
                    };
                }
                Token::GreaterEqual => {
                    self.adv();
                    let r = self.shift()?;
                    l = Expr::Binary {
                        op: ">=".to_string(),
                        left: Box::new(l),
                        right: Box::new(r),
                    };
                }
                Token::KwInstanceof => {
                    self.adv();
                    let r = self.shift()?;
                    l = Expr::Binary {
                        op: "instanceof".to_string(),
                        left: Box::new(l),
                        right: Box::new(r),
                    };
                }
                Token::KwIn => {
                    self.adv();
                    let r = self.shift()?;
                    l = Expr::Binary {
                        op: "in".to_string(),
                        left: Box::new(l),
                        right: Box::new(r),
                    };
                }
                _ => break,
            }
        }
        Some(l)
    }

    fn shift(&mut self) -> Option<Expr> {
        let mut l = self.add()?;
        loop {
            let op = match self.cur() {
                Token::Shl => "<<",
                Token::Shr => ">>",
                Token::UShr => ">>>",
                _ => break,
            };
            self.adv();
            let r = self.add()?;
            l = Expr::Binary {
                op: op.to_string(),
                left: Box::new(l),
                right: Box::new(r),
            };
        }
        Some(l)
    }

    fn add(&mut self) -> Option<Expr> {
        let mut l = self.mul()?;
        loop {
            match self.cur() {
                Token::Plus => {
                    self.adv();
                    let r = self.mul()?;
                    l = Expr::Binary {
                        op: "+".to_string(),
                        left: Box::new(l),
                        right: Box::new(r),
                    };
                }
                Token::Minus => {
                    self.adv();
                    let r = self.mul()?;
                    l = Expr::Binary {
                        op: "-".to_string(),
                        left: Box::new(l),
                        right: Box::new(r),
                    };
                }
                _ => break,
            }
        }
        Some(l)
    }

    fn mul(&mut self) -> Option<Expr> {
        let mut l = self.exponent()?;
        loop {
            match self.cur() {
                Token::Star => {
                    self.adv();
                    let r = self.exponent()?;
                    l = Expr::Binary {
                        op: "*".to_string(),
                        left: Box::new(l),
                        right: Box::new(r),
                    };
                }
                Token::Slash => {
                    self.adv();
                    let r = self.exponent()?;
                    l = Expr::Binary {
                        op: "/".to_string(),
                        left: Box::new(l),
                        right: Box::new(r),
                    };
                }
                Token::Percent => {
                    self.adv();
                    let r = self.exponent()?;
                    l = Expr::Binary {
                        op: "%".to_string(),
                        left: Box::new(l),
                        right: Box::new(r),
                    };
                }
                _ => break,
            }
        }
        Some(l)
    }

    fn exponent(&mut self) -> Option<Expr> {
        let base = self.unary()?;
        if self.eat(&Token::StarStar) {
            // Right-associative: 2 ** 3 ** 2 === 2 ** (3 ** 2)
            let exp = self.exponent()?;
            Some(Expr::Binary {
                op: "**".to_string(),
                left: Box::new(base),
                right: Box::new(exp),
            })
        } else {
            Some(base)
        }
    }

    fn unary(&mut self) -> Option<Expr> {
        match self.cur() {
            Token::Not => {
                self.adv();
                let o = self.unary()?;
                Some(Expr::Unary {
                    op: "!".to_string(),
                    operand: Box::new(o),
                    prefix: true,
                })
            }
            Token::Tilde => {
                self.adv();
                let o = self.unary()?;
                Some(Expr::Unary {
                    op: "~".to_string(),
                    operand: Box::new(o),
                    prefix: true,
                })
            }
            Token::Minus => {
                self.adv();
                let o = self.unary()?;
                Some(Expr::Unary {
                    op: "-".to_string(),
                    operand: Box::new(o),
                    prefix: true,
                })
            }
            Token::Plus => {
                self.adv();
                let o = self.unary()?;
                Some(Expr::Unary {
                    op: "+".to_string(),
                    operand: Box::new(o),
                    prefix: true,
                })
            }
            Token::KwTypeof => {
                self.adv();
                let o = self.unary()?;
                Some(Expr::Unary {
                    op: "typeof".to_string(),
                    operand: Box::new(o),
                    prefix: true,
                })
            }
            Token::KwVoid => {
                self.adv();
                let o = self.unary()?;
                Some(Expr::Unary {
                    op: "void".to_string(),
                    operand: Box::new(o),
                    prefix: true,
                })
            }
            Token::KwDelete => {
                self.adv();
                let o = self.unary()?;
                Some(Expr::Unary {
                    op: "delete".to_string(),
                    operand: Box::new(o),
                    prefix: true,
                })
            }
            Token::PlusPlus => {
                self.adv();
                let o = self.unary()?;
                Some(Expr::Unary {
                    op: "++".to_string(),
                    operand: Box::new(o),
                    prefix: true,
                })
            }
            Token::MinusMinus => {
                self.adv();
                let o = self.unary()?;
                Some(Expr::Unary {
                    op: "--".to_string(),
                    operand: Box::new(o),
                    prefix: true,
                })
            }
            _ => self.postfix(),
        }
    }

    fn postfix(&mut self) -> Option<Expr> {
        let mut e = self.primary()?;
        loop {
            match self.cur() {
                Token::LParen => {
                    self.adv();
                    let mut a = Vec::new();
                    while !matches!(self.cur(), Token::RParen) {
                        if let Some(arg) = self.assign() {
                            a.push(arg);
                        } else {
                            self.adv();
                        }
                        if !matches!(self.cur(), Token::RParen) {
                            self.eat(&Token::Comma);
                        }
                    }
                    self.eat(&Token::RParen);
                    e = Expr::Call {
                        callee: Box::new(e),
                        args: a,
                    };
                }
                Token::Dot => {
                    self.adv();
                    if self.eat(&Token::QuestionDot) {
                        // optional chaining: obj?.prop
                        let p = self.ident()?;
                        e = Expr::OptionalChain {
                            object: Box::new(e),
                            property: Box::new(Expr::String(p)),
                            computed: false,
                        };
                    } else {
                        let p = self.ident()?;
                        e = Expr::Member {
                            object: Box::new(e),
                            property: Box::new(Expr::String(p)),
                            computed: false,
                        };
                    }
                }
                Token::LBracket => {
                    self.adv();
                    let p = self.expr()?;
                    self.eat(&Token::RBracket);
                    e = Expr::Member {
                        object: Box::new(e),
                        property: Box::new(p),
                        computed: true,
                    };
                }
                Token::PlusPlus => {
                    self.adv();
                    e = Expr::Unary {
                        op: "++".to_string(),
                        operand: Box::new(e),
                        prefix: false,
                    };
                }
                Token::MinusMinus => {
                    self.adv();
                    e = Expr::Unary {
                        op: "--".to_string(),
                        operand: Box::new(e),
                        prefix: false,
                    };
                }
                Token::QuestionDot => {
                    self.adv();
                    if self.eat(&Token::LParen) {
                        // Optional call: obj?.(args)
                        let mut a = Vec::new();
                        while !matches!(self.cur(), Token::RParen) {
                            if let Some(arg) = self.assign() {
                                a.push(arg);
                            } else {
                                self.adv();
                            }
                            if !matches!(self.cur(), Token::RParen) {
                                self.eat(&Token::Comma);
                            }
                        }
                        self.eat(&Token::RParen);
                        e = Expr::Call {
                            callee: Box::new(Expr::OptionalChain {
                                object: Box::new(e),
                                property: Box::new(Expr::Undefined),
                                computed: false,
                            }),
                            args: a,
                        };
                    } else if self.eat(&Token::LBracket) {
                        // Optional computed member: obj?.[expr]
                        let p = self.assign()?;
                        self.eat(&Token::RBracket);
                        e = Expr::OptionalChain {
                            object: Box::new(e),
                            property: Box::new(p),
                            computed: true,
                        };
                    } else {
                        // Optional member: obj?.prop
                        let p = self.ident()?;
                        e = Expr::OptionalChain {
                            object: Box::new(e),
                            property: Box::new(Expr::String(p)),
                            computed: false,
                        };
                    }
                }
                Token::Arrow => {
                    if let Expr::Identifier(n) = e {
                        self.adv();
                        e = self.arrow_body(vec![n], vec![]);
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
        Some(e)
    }

    fn primary(&mut self) -> Option<Expr> {
        match self.cur() {
            Token::Number(n) => {
                let v = *n;
                self.adv();
                Some(Expr::Number(v))
            }
            Token::String(s) => {
                let v = s.clone();
                self.adv();
                Some(Expr::String(v))
            }
            Token::KwTrue => {
                self.adv();
                Some(Expr::Bool(true))
            }
            Token::KwFalse => {
                self.adv();
                Some(Expr::Bool(false))
            }
            Token::KwNull => {
                self.adv();
                Some(Expr::Null)
            }
            Token::KwUndefined => {
                self.adv();
                Some(Expr::Undefined)
            }
            Token::KwThis => {
                self.adv();
                Some(Expr::This)
            }
            Token::Backtick => {
                self.adv();
                let mut quasis = Vec::new();
                let mut exprs = Vec::new();
                // Leading quasi (possibly empty).
                quasis.push(self.take_quasi());
                while matches!(self.cur(), Token::DollarLBrace) {
                    self.adv();
                    exprs.push(self.expr()?);
                    self.eat(&Token::RBrace);
                    quasis.push(self.take_quasi());
                }
                self.eat(&Token::Backtick);
                Some(Expr::Template { quasis, exprs })
            }
            Token::LParen => {
                // Speculatively try to parse an arrow-function parameter list.
                if let Some(arrow) = self.try_arrow() {
                    return Some(arrow);
                }
                // Otherwise it is a parenthesized expression.
                self.adv();
                let e = self.expr()?;
                self.eat(&Token::RParen);
                Some(e)
            }
            Token::LBracket => {
                self.adv();
                let mut i = Vec::new();
                while !matches!(self.cur(), Token::RBracket) {
                    if self.eat(&Token::Comma) {
                        i.push(Expr::Undefined);
                        continue;
                    }
                    if self.eat(&Token::DotDotDot) {
                        i.push(Expr::Spread(Box::new(self.assign()?)));
                    } else {
                        i.push(self.assign()?);
                    }
                    if !matches!(self.cur(), Token::RBracket) {
                        self.eat(&Token::Comma);
                    }
                }
                self.eat(&Token::RBracket);
                Some(Expr::Array(i))
            }
            Token::LBrace => {
                self.adv();
                let mut p = Vec::new();
                while !matches!(self.cur(), Token::RBrace) {
                    if self.eat(&Token::DotDotDot) {
                        let s = self.assign()?;
                        p.push(ObjectProp::Spread(s));
                        if !matches!(self.cur(), Token::RBrace) {
                            self.eat(&Token::Comma);
                        }
                        continue;
                    }
                    let is_method = self.eat(&Token::KwGet);
                    let is_setter = self.eat(&Token::KwSet);
                    let key = match self.cur() {
                        Token::Identifier(n) => {
                            let v = n.clone();
                            self.adv();
                            v
                        }
                        Token::String(s) => {
                            let v = s.clone();
                            self.adv();
                            v
                        }
                        Token::Number(n) => {
                            let v = n.to_string();
                            self.adv();
                            v
                        }
                        Token::LBracket => {
                            self.adv();
                            let e = self.assign()?;
                            self.eat(&Token::RBracket);
                            match self.cur() {
                                Token::Colon => {
                                    self.adv();
                                    let v = self.assign()?;
                                    p.push(ObjectProp::Computed(e, v));
                                    if !matches!(self.cur(), Token::RBrace) {
                                        self.eat(&Token::Comma);
                                    }
                                    continue;
                                }
                                Token::LParen => {
                                    let key_str = match e {
                                        Expr::String(s) => s,
                                        Expr::Number(n) => n.to_string(),
                                        _ => return None,
                                    };
                                    self.adv();
                                    let (params, defaults) = self.params();
                                    self.eat(&Token::RParen);
                                    self.eat(&Token::LBrace);
                                    let b = self.block_body();
                                    self.eat(&Token::RBrace);
                                    let mut body = defaults;
                                    body.extend(b);
                                    if is_method {
                                        p.push(ObjectProp::Getter {
                                            name: key_str,
                                            body,
                                        });
                                    } else if is_setter {
                                        let param = params.first().cloned().unwrap_or_default();
                                        p.push(ObjectProp::Setter {
                                            name: key_str,
                                            param,
                                            body,
                                        });
                                    } else {
                                        p.push(ObjectProp::Method {
                                            name: key_str,
                                            params,
                                            body,
                                        });
                                    }
                                    if !matches!(self.cur(), Token::RBrace) {
                                        self.eat(&Token::Comma);
                                    }
                                    continue;
                                }
                                _ => return None,
                            }
                        }
                        _ => break,
                    };
                    if is_setter {
                        self.eat(&Token::LParen);
                        let param = self.ident()?;
                        self.eat(&Token::RParen);
                        self.eat(&Token::LBrace);
                        let b = self.block_body();
                        self.eat(&Token::RBrace);
                        p.push(ObjectProp::Setter {
                            name: key,
                            param,
                            body: b,
                        });
                    } else if self.eat(&Token::LParen) {
                        let (params, defaults) = self.params();
                        self.eat(&Token::RParen);
                        self.eat(&Token::LBrace);
                        let b = self.block_body();
                        self.eat(&Token::RBrace);
                        let mut body = defaults;
                        body.extend(b);
                        if is_method {
                            p.push(ObjectProp::Getter {
                                name: key,
                                body,
                            });
                        } else {
                            p.push(ObjectProp::Method {
                                name: key,
                                params,
                                body,
                            });
                        }
                    } else if self.eat(&Token::Colon) {
                        let v = self.assign()?;
                        p.push(ObjectProp::KeyValue(key, v));
                    } else {
                        p.push(ObjectProp::Shorthand(key));
                    }
                    if !matches!(self.cur(), Token::RBrace) {
                        self.eat(&Token::Comma);
                    }
                }
                self.eat(&Token::RBrace);
                Some(Expr::Object(p))
            }
            Token::KwFunction => {
                self.adv();
                let n = if let Token::Identifier(x) = self.cur() {
                    let v = x.clone();
                    self.adv();
                    Some(v)
                } else {
                    None
                };
                self.eat(&Token::LParen);
                let (p, defaults) = self.params();
                self.eat(&Token::RParen);
                self.eat(&Token::LBrace);
                let b = self.block_body();
                self.eat(&Token::RBrace);
                let mut body = defaults;
                body.extend(b);
                Some(Expr::FnExpr {
                    name: n,
                    params: p,
                    body,
                })
            }
            Token::KwNew => {
                self.adv();
                let c = self.new_callee()?;
                let a = if self.eat(&Token::LParen) {
                    let mut ag = Vec::new();
                    while !matches!(self.cur(), Token::RParen) {
                        if let Some(arg) = self.assign() {
                            ag.push(arg);
                        } else {
                            self.adv();
                        }
                        if !matches!(self.cur(), Token::RParen) {
                            self.eat(&Token::Comma);
                        }
                    }
                    self.eat(&Token::RParen);
                    ag
                } else {
                    vec![]
                };
                Some(Expr::New {
                    callee: Box::new(c),
                    args: a,
                })
            }
            Token::KwImport => {
                self.adv();
                if self.eat(&Token::Dot)
                    && let Token::Identifier(m) = self.cur()
                    && m == "meta"
                {
                    self.adv();
                    return Some(Expr::ImportMeta);
                }
                self.semi();
                Some(Expr::Undefined)
            }
            Token::Identifier(n) => {
                let nm = n.clone();
                self.adv();
                Some(Expr::Identifier(nm))
            }
            Token::DotDotDot => {
                self.adv();
                let i = self.assign()?;
                Some(Expr::Spread(Box::new(i)))
            }
            _ => None,
        }
    }

    /// Parses the callee of a `new` expression: a primary expression followed
    /// by member access (dot / computed), but stopping before call arguments so
    /// that `new Foo(1, 2)` treats `(1, 2)` as the constructor's arguments.
    fn new_callee(&mut self) -> Option<Expr> {
        let mut e = self.primary()?;
        loop {
            match self.cur() {
                Token::Dot => {
                    self.adv();
                    let p = self.ident()?;
                    e = Expr::Member {
                        object: Box::new(e),
                        property: Box::new(Expr::String(p)),
                        computed: false,
                    };
                }
                Token::LBracket => {
                    self.adv();
                    let p = self.expr()?;
                    self.eat(&Token::RBracket);
                    e = Expr::Member {
                        object: Box::new(e),
                        property: Box::new(p),
                        computed: true,
                    };
                }
                _ => break,
            }
        }
        Some(e)
    }

    /// Consume a `TemplateQuasi` token if present, returning its raw text
    /// (empty string when the quasi is absent).
    fn take_quasi(&mut self) -> String {
        if let Token::TemplateQuasi(q) = self.cur() {
            let s = q.clone();
            self.adv();
            s
        } else {
            String::new()
        }
    }

    /// Speculatively parse `( params ) =>`. On any failure, restore the parser
    /// position and return `None` so the caller can parse a parenthesized expr.
    fn try_arrow(&mut self) -> Option<Expr> {
        let save = self.pos;
        if !self.eat(&Token::LParen) {
            return None;
        }
        let mut params = Vec::new();
        let mut defaults = Vec::new();
        if self.eat(&Token::RParen) {
            if self.eat(&Token::Arrow) {
                return Some(self.arrow_body(params, defaults));
            }
            self.pos = save;
            return None;
        }
        loop {
            match self.cur() {
                Token::DotDotDot => {
                    self.adv();
                    if let Token::Identifier(n) = self.cur() {
                        params.push(format!("...{}", n));
                        self.adv();
                    } else {
                        self.pos = save;
                        return None;
                    }
                }
                Token::Identifier(n) => {
                    let name = n.clone();
                    self.adv();
                    if self.eat(&Token::Equal) {
                        match self.assign() {
                            Some(d) => defaults.push(Parser::default_guard(&name, d)),
                            None => {
                                self.pos = save;
                                return None;
                            }
                        }
                    }
                    params.push(name);
                }
                _ => {
                    self.pos = save;
                    return None;
                }
            }
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        if !self.eat(&Token::RParen) || !self.eat(&Token::Arrow) {
            self.pos = save;
            return None;
        }
        Some(self.arrow_body(params, defaults))
    }

    fn arrow_body(&mut self, params: Vec<String>, defaults: Vec<Statement>) -> Expr {
        if self.eat(&Token::LBrace) {
            let b = self.block_body();
            self.eat(&Token::RBrace);
            let mut body = defaults;
            body.extend(b);
            Expr::ArrowFn {
                params,
                body: Box::new(ExprOrBlock::Block(body)),
            }
        } else {
            let e = self.assign().unwrap_or(Expr::Undefined);
            if defaults.is_empty() {
                Expr::ArrowFn {
                    params,
                    body: Box::new(ExprOrBlock::Expr(Box::new(e))),
                }
            } else {
                let mut body = defaults;
                body.push(Statement::Return(Some(Box::new(e))));
                Expr::ArrowFn {
                    params,
                    body: Box::new(ExprOrBlock::Block(body)),
                }
            }
        }
    }
}
