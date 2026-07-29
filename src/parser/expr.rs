use super::{Expr, ObjectProp, Parser};
use crate::lexer::Token;

impl Parser {
    pub(crate) fn expr(&mut self) -> Option<Expr> {
        self.assign()
    }

    fn assign(&mut self) -> Option<Expr> {
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
        let t = self.comma()?;
        if self.eat(&Token::Question) {
            let c = self.expr()?;
            self.eat(&Token::Colon);
            let a = self.expr()?;
            Some(Expr::Conditional {
                test: Box::new(t),
                consequent: Box::new(c),
                alternate: Box::new(a),
            })
        } else {
            Some(t)
        }
    }

    fn comma(&mut self) -> Option<Expr> {
        let mut l = self.nullish()?;
        while self.eat(&Token::Comma) {
            let r = self.nullish()?;
            l = Expr::Binary {
                op: ",".to_string(),
                left: Box::new(l),
                right: Box::new(r),
            };
        }
        Some(l)
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
                        if let Some(arg) = self.expr() {
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
                        let mut a = Vec::new();
                        while !matches!(self.cur(), Token::RParen) {
                            if let Some(arg) = self.expr() {
                                a.push(arg);
                            } else {
                                self.adv();
                            }
                            if !matches!(self.cur(), Token::RParen) {
                                self.eat(&Token::Comma);
                            }
                        }
                        self.eat(&Token::RParen);
                        e = Expr::OptionalChain {
                            object: Box::new(e),
                            property: Box::new(Expr::Undefined),
                            computed: false,
                        };
                    } else {
                        break;
                    }
                }
                Token::Arrow => {
                    if let Expr::Identifier(n) = e {
                        self.adv();
                        e = self.arrow_body(&[n]);
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
                let mut current = String::new();
                while !matches!(self.cur(), Token::Backtick) && !self.eof() {
                    match self.cur() {
                        Token::DollarLBrace => {
                            self.adv();
                            quasis.push(current);
                            current = String::new();
                            exprs.push(self.expr()?);
                            self.eat(&Token::RBrace);
                        }
                        _ => {
                            let c = match self.cur() {
                                Token::String(s) => s.clone(),
                                Token::Number(n) => n.to_string(),
                                Token::Identifier(s) => s.clone(),
                                _ => format!("{:?}", self.cur()),
                            };
                            current.push_str(&c);
                            self.adv();
                        }
                    }
                }
                if self.eat(&Token::Backtick) {
                    quasis.push(current);
                }
                Some(Expr::Template { quasis, exprs })
            }
            Token::LParen => {
                self.adv();
                if self.eat(&Token::RParen) {
                    if self.eat(&Token::Arrow) {
                        return Some(self.arrow_body(&[]));
                    }
                    return Some(Expr::Undefined);
                }
                let f = self.expr()?;
                if self.eat(&Token::RParen) {
                    if self.eat(&Token::Arrow) {
                        let n = match &f {
                            Expr::Identifier(x) => x.clone(),
                            _ => return None,
                        };
                        return Some(self.arrow_body(&[n]));
                    }
                    return Some(f);
                }
                if self.eat(&Token::Comma) {
                    let mut p = vec![];
                    if let Expr::Identifier(n) = f {
                        p.push(n);
                    }
                    while self.eat(&Token::Comma) {
                        if let Token::Identifier(x) = self.cur() {
                            p.push(x.clone());
                            self.adv();
                        }
                    }
                    self.eat(&Token::RParen);
                    if self.eat(&Token::Arrow) {
                        return Some(self.arrow_body(&p));
                    }
                    return None;
                }
                let e = self.assign()?;
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
                        i.push(Expr::Spread(Box::new(self.expr()?)));
                    } else {
                        i.push(self.expr()?);
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
                        let s = self.expr()?;
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
                            let e = self.expr()?;
                            self.eat(&Token::RBracket);
                            match self.cur() {
                                Token::Colon => {
                                    self.adv();
                                    let v = self.expr()?;
                                    let key_expr = match e {
                                        Expr::String(s) => s,
                                        Expr::Number(n) => n.to_string(),
                                        _ => return None,
                                    };
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
                                    let params = self.params();
                                    self.eat(&Token::RParen);
                                    self.eat(&Token::LBrace);
                                    let b = self.block_body();
                                    self.eat(&Token::RBrace);
                                    if is_getter {
                                        p.push(ObjectProp::Getter {
                                            name: key_str,
                                            body: b,
                                        });
                                    } else if is_setter {
                                        let param = params.first().cloned().unwrap_or_default();
                                        p.push(ObjectProp::Setter {
                                            name: key_str,
                                            param,
                                            body: b,
                                        });
                                    } else {
                                        p.push(ObjectProp::Method {
                                            name: key_str,
                                            params,
                                            body: b,
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
                        let params = self.params();
                        self.eat(&Token::RParen);
                        self.eat(&Token::LBrace);
                        let b = self.block_body();
                        self.eat(&Token::RBrace);
                        if is_getter {
                            p.push(ObjectProp::Getter {
                                name: key,
                                body: b,
                            });
                        } else {
                            p.push(ObjectProp::Method {
                                name: key,
                                params,
                                body: b,
                            });
                        }
                    } else if self.eat(&Token::Colon) {
                        let v = self.expr()?;
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
                let p = self.params();
                self.eat(&Token::RParen);
                self.eat(&Token::LBrace);
                let b = self.block_body();
                self.eat(&Token::RBrace);
                Some(Expr::FnExpr {
                    name: n,
                    params: p,
                    body: b,
                })
            }
            Token::KwNew => {
                self.adv();
                let c = self.expr()?;
                let a = if self.eat(&Token::LParen) {
                    let mut ag = Vec::new();
                    while !matches!(self.cur(), Token::RParen) {
                        if let Some(arg) = self.expr() {
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
                let i = self.expr()?;
                Some(Expr::Spread(Box::new(i)))
            }
            _ => None,
        }
    }

    fn arrow_body(&mut self, p: &[String]) -> Expr {
        if self.eat(&Token::LBrace) {
            let b = self.block_body();
            self.eat(&Token::RBrace);
            Expr::ArrowFn {
                params: p.to_vec(),
                body: Box::new(ExprOrBlock::Block(b)),
            }
        } else {
            let e = self.expr().unwrap_or(Expr::Undefined);
            Expr::ArrowFn {
                params: p.to_vec(),
                body: Box::new(ExprOrBlock::Expr(Box::new(e))),
            }
        }
    }
}
