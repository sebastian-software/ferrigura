//! The ER grammar of `erDiagram.jison` as recursive descent over the scanner's tokens. The DB
//! calls are made where the grammar's actions run, in the order of the reductions, and the value
//! of a statement is the list of members the enclosing subgraph keeps (`docs-port/er/PLAN.md`,
//! sections 4 and 4.1).

use ferrigura_model::er::{Cardinality, ErAttribute, ErDiagram, Identification, RelSpec};

use super::db::{ErDb, Member};
use super::scanner::{Kind, Token};
use crate::error::{ParseError, Source};
use crate::text::js_trim;

/// The parser: the tokens, a position, and the DB the actions write to.
pub(crate) struct Parser<'a> {
    source: Source<'a>,
    tokens: Vec<Token>,
    pos: usize,
    db: ErDb,
}

impl<'a> Parser<'a> {
    pub(crate) fn new(source: Source<'a>, tokens: Vec<Token>, db: ErDb) -> Self {
        Self {
            source,
            tokens,
            pos: 0,
            db,
        }
    }

    /// The model of the DB after the document has been parsed.
    pub(crate) fn into_model(self) -> ErDiagram {
        self.db.into_model()
    }

    /// `start: 'ER_DIAGRAM' document 'EOF'`.
    pub(crate) fn parse_document(&mut self) -> Result<(), ParseError> {
        if self.kind() != Kind::ErDiagram {
            return Err(self.error_here("expected erDiagram"));
        }
        self.bump();
        self.document(false)?;
        self.expect(Kind::Eof)?;
        Ok(())
    }

    fn kind(&self) -> Kind {
        self.tokens
            .get(self.pos)
            .map_or(Kind::End, |token| token.kind)
    }

    /// Moves past the current token; the end marker stays where it is.
    fn bump(&mut self) {
        if self.kind() != Kind::End {
            self.pos += 1;
        }
    }

    /// Takes the current token and moves past it.
    fn take(&mut self) -> Result<Token, ParseError> {
        let token = self
            .tokens
            .get(self.pos)
            .cloned()
            .ok_or_else(|| self.error_here("unexpected end of the diagram"))?;
        self.bump();
        Ok(token)
    }

    /// Takes the current token if it is of `kind`, and fails otherwise.
    fn expect(&mut self, kind: Kind) -> Result<Token, ParseError> {
        if self.kind() == kind {
            self.take()
        } else {
            Err(self.error_here("unexpected token"))
        }
    }

    /// Moves past the current token if it is of `kind`.
    fn eat(&mut self, kind: Kind) -> bool {
        let found = self.kind() == kind;
        if found {
            self.bump();
        }
        found
    }

    fn error_here(&self, message: &str) -> ParseError {
        let offset = self
            .tokens
            .get(self.pos)
            .map_or(self.source.text().len(), |token| token.start);
        self.source.error(offset, message)
    }

    /// `document`: the lines up to the `end` of a subgraph, or up to the end of the diagram. The
    /// members of the lines are returned in order.
    fn document(&mut self, in_subgraph: bool) -> Result<Vec<Member>, ParseError> {
        let mut members = Vec::new();
        loop {
            match self.kind() {
                Kind::Newline => self.bump(),
                Kind::Eof if !in_subgraph => return Ok(members),
                Kind::KwEnd if in_subgraph => return Ok(members),
                Kind::Eof | Kind::KwEnd | Kind::End => {
                    return Err(self.error_here("unexpected end of the diagram"));
                }
                _ => members.extend(self.statement()?),
            }
        }
    }

    /// `statement`: one statement, which the grammar's alternatives give.
    fn statement(&mut self) -> Result<Vec<Member>, ParseError> {
        match self.kind() {
            Kind::EntityName
            | Kind::UnicodeText
            | Kind::Num
            | Kind::DecimalNum
            | Kind::EntityOne => self.entity_statement(),
            Kind::AccTitle => {
                self.bump();
                let value = js_trim(&self.expect(Kind::AccTitleValue)?.value).to_owned();
                self.db.set_acc_title(&value);
                Ok(vec![Member::from(value.as_str())])
            }
            Kind::AccDescr => {
                self.bump();
                let value = js_trim(&self.expect(Kind::AccDescrValue)?.value).to_owned();
                self.db.set_acc_description(&value);
                Ok(vec![Member::from(value.as_str())])
            }
            Kind::AccDescrMultiValue => {
                let value = js_trim(&self.take()?.value).to_owned();
                self.db.set_acc_description(&value);
                Ok(vec![Member::from(value.as_str())])
            }
            Kind::DirectionTb | Kind::DirectionBt | Kind::DirectionRl | Kind::DirectionLr => {
                let direction = match self.take()?.kind {
                    Kind::DirectionTb => "TB",
                    Kind::DirectionBt => "BT",
                    Kind::DirectionRl => "RL",
                    _ => "LR",
                };
                if self.db.subgraph_depth() == 0 {
                    self.db.set_direction(direction);
                    Ok(Vec::new())
                } else {
                    Ok(vec![Member::Direction(direction.to_owned())])
                }
            }
            Kind::ClassDef => self.class_def(),
            Kind::Class => self.class_statement(),
            Kind::Style => self.style_statement(),
            Kind::Subgraph => self.subgraph_statement(),
            _ => Err(self.error_here("expected a statement")),
        }
    }

    /// `entityName relSpec entityName COLON role` and the forms with `:::` classes, the alias
    /// forms, the attribute blocks, and the plain entity. The entity's name is the first member.
    fn entity_statement(&mut self) -> Result<Vec<Member>, ParseError> {
        let first = self.entity_name()?;
        let first_classes = self.optional_classes()?;
        if is_cardinality(self.kind()) {
            let spec = self.rel_spec()?;
            let second = self.entity_name()?;
            let second_classes = self.optional_classes()?;
            self.expect(Kind::Colon)?;
            let role = self.role()?;
            self.db.add_entity(&first, "");
            self.db.add_entity(&second, "");
            self.db.add_relationship(&first, &role, &second, spec);
            if let Some(classes) = first_classes {
                self.db.set_class(std::slice::from_ref(&first), &classes);
            }
            if let Some(classes) = second_classes {
                self.db.set_class(std::slice::from_ref(&second), &classes);
            }
            return Ok(vec![Member::Name(first), Member::Name(second)]);
        }
        if first_classes.is_none() && self.eat(Kind::Sqs) {
            let alias = self.entity_name()?;
            self.expect(Kind::Sqe)?;
            let classes = self.optional_classes()?;
            self.db.add_entity(&first, &alias);
            if self.eat(Kind::BlockStart) {
                self.attributes_into(&first)?;
            }
            if let Some(classes) = classes {
                self.db.set_class(std::slice::from_ref(&first), &classes);
            }
            return Ok(vec![Member::Name(first)]);
        }
        if self.eat(Kind::BlockStart) {
            self.db.add_entity(&first, "");
            self.attributes_into(&first)?;
            if let Some(classes) = first_classes {
                self.db.set_class(std::slice::from_ref(&first), &classes);
            }
            return Ok(vec![Member::Name(first)]);
        }
        self.db.add_entity(&first, "");
        if let Some(classes) = first_classes {
            self.db.set_class(std::slice::from_ref(&first), &classes);
        }
        Ok(vec![Member::Name(first)])
    }

    /// `:::` and its class list, when the current token is `:::`.
    fn optional_classes(&mut self) -> Result<Option<Vec<String>>, ParseError> {
        if self.eat(Kind::StyleSeparator) {
            Ok(Some(self.id_list(Kind::UnicodeText)?))
        } else {
            Ok(None)
        }
    }

    /// The attributes of a block, after its `{`, added to the entity. An empty block adds none.
    fn attributes_into(&mut self, name: &str) -> Result<(), ParseError> {
        let attributes = self.attributes()?;
        if !attributes.is_empty() {
            self.db.add_attributes(name, attributes);
        }
        Ok(())
    }

    /// `attributes`: one or more attributes up to the closing brace, which is consumed.
    fn attributes(&mut self) -> Result<Vec<ErAttribute>, ParseError> {
        let mut attributes = Vec::new();
        loop {
            match self.kind() {
                Kind::BlockStop => {
                    self.bump();
                    return Ok(attributes);
                }
                Kind::AttributeWord => attributes.push(self.attribute()?),
                _ => return Err(self.error_here("expected an attribute or the end of the block")),
            }
        }
    }

    /// `attributeType attributeName [keys] [comment]`.
    fn attribute(&mut self) -> Result<ErAttribute, ParseError> {
        let mut attr_type = self.take()?.value;
        if self.eat(Kind::Literal('?')) {
            attr_type.push('?');
        }
        let name = self.expect(Kind::AttributeWord)?.value;
        let mut keys = Vec::new();
        if self.kind() == Kind::AttributeKey {
            keys.push(self.take()?.value);
            while self.eat(Kind::Literal(',')) {
                keys.push(self.expect(Kind::AttributeKey)?.value);
            }
        }
        let comment = if self.kind() == Kind::Comment {
            self.take()?.value.replace('"', "")
        } else {
            String::new()
        };
        Ok(ErAttribute {
            attr_type,
            name,
            keys,
            comment,
        })
    }

    /// `entityName`: a quoted name without its quotes, or a name as written.
    fn entity_name(&mut self) -> Result<String, ParseError> {
        match self.kind() {
            Kind::EntityName
            | Kind::UnicodeText
            | Kind::Num
            | Kind::DecimalNum
            | Kind::EntityOne => {
                let token = self.take()?;
                Ok(match token.kind {
                    Kind::EntityName => token.value.replace('"', ""),
                    _ => token.value,
                })
            }
            _ => Err(self.error_here("expected an entity name")),
        }
    }

    /// `role`: a quoted text without its quotes, or a name as written.
    fn role(&mut self) -> Result<String, ParseError> {
        match self.kind() {
            Kind::Word | Kind::EntityName => Ok(self.take()?.value.replace('"', "")),
            Kind::UnicodeText => Ok(self.take()?.value),
            _ => Err(self.error_here("expected a role")),
        }
    }

    /// `relSpec: cardinality relType cardinality`. The right-hand cardinality is `cardA`.
    fn rel_spec(&mut self) -> Result<RelSpec, ParseError> {
        let card_b = self.cardinality()?;
        let rel_type = match self.kind() {
            Kind::NonIdentifying => Identification::NonIdentifying,
            Kind::Identifying => Identification::Identifying,
            _ => return Err(self.error_here("expected a relationship type")),
        };
        self.bump();
        let card_a = self.cardinality()?;
        Ok(RelSpec {
            card_a,
            rel_type,
            card_b,
        })
    }

    fn cardinality(&mut self) -> Result<Cardinality, ParseError> {
        let card = match self.kind() {
            Kind::ZeroOrOne => Cardinality::ZeroOrOne,
            Kind::ZeroOrMore => Cardinality::ZeroOrMore,
            Kind::OneOrMore => Cardinality::OneOrMore,
            Kind::OnlyOne => Cardinality::OnlyOne,
            Kind::MdParent => Cardinality::MdParent,
            _ => return Err(self.error_here("expected a cardinality")),
        };
        self.bump();
        Ok(card)
    }

    /// `idList`: one or more tokens of `kind`, separated by commas.
    fn id_list(&mut self, kind: Kind) -> Result<Vec<String>, ParseError> {
        let mut ids = vec![self.expect(kind)?.value];
        while self.eat(Kind::Comma) {
            ids.push(self.expect(kind)?.value);
        }
        Ok(ids)
    }

    /// `classDef idList stylesOpt separator`. The statement's value is the keyword as written.
    fn class_def(&mut self) -> Result<Vec<Member>, ParseError> {
        let keyword = self.take()?.value;
        let ids = self.id_list(Kind::StyleText)?;
        let styles = self.styles()?;
        self.style_separator()?;
        self.db.add_class(&ids, &styles);
        Ok(vec![Member::Name(keyword)])
    }

    /// `class idList idList`: the first list names the entities, the second the classes.
    fn class_statement(&mut self) -> Result<Vec<Member>, ParseError> {
        let keyword = self.take()?.value;
        let ids = self.id_list(Kind::UnicodeText)?;
        let names = self.id_list(Kind::UnicodeText)?;
        self.db.set_class(&ids, &names);
        Ok(vec![Member::Name(keyword)])
    }

    /// `style idList stylesOpt separator`.
    fn style_statement(&mut self) -> Result<Vec<Member>, ParseError> {
        let keyword = self.take()?.value;
        let ids = self.id_list(Kind::StyleText)?;
        let styles = self.styles()?;
        self.style_separator()?;
        self.db.add_css_styles(&ids, &styles);
        Ok(vec![Member::Name(keyword)])
    }

    /// `stylesOpt`: one or more styles, separated by commas.
    fn styles(&mut self) -> Result<Vec<String>, ParseError> {
        let mut styles = vec![self.style()?];
        while self.eat(Kind::Comma) {
            styles.push(self.style()?);
        }
        Ok(styles)
    }

    /// `style: styleComponent+`, concatenated without blanks.
    fn style(&mut self) -> Result<String, ParseError> {
        let mut text = String::new();
        while matches!(self.kind(), Kind::StyleText | Kind::Colon | Kind::Brkt) {
            text.push_str(&self.take()?.value);
        }
        if text.is_empty() {
            return Err(self.error_here("expected a style"));
        }
        Ok(text)
    }

    /// `separator` in the style state: `;` or a line break.
    fn style_separator(&mut self) -> Result<(), ParseError> {
        match self.kind() {
            Kind::Semi | Kind::Newline => {
                self.bump();
                Ok(())
            }
            _ => Err(self.error_here("expected the end of the style statement")),
        }
    }

    /// `subgraphHeader document END`. The depth counts the subgraph from its header, and the
    /// statement's value is the subgraph id.
    fn subgraph_statement(&mut self) -> Result<Vec<Member>, ParseError> {
        self.bump();
        let id = self.entity_name()?;
        let title = if self.eat(Kind::Sqs) {
            let title = self.subgraph_title()?;
            self.expect(Kind::Sqe)?;
            title
        } else {
            id.clone()
        };
        self.expect(Kind::Newline)?;
        self.db.open_subgraph();
        let members = self.document(true)?;
        self.expect(Kind::KwEnd)?;
        self.db.close_subgraph();
        let id = self.db.add_subgraph(&id, &members, &title);
        Ok(vec![Member::Name(id)])
    }

    /// `subgraphTitle`: entity names, joined by one space.
    fn subgraph_title(&mut self) -> Result<String, ParseError> {
        let mut words = vec![self.entity_name()?];
        while is_entity_start(self.kind()) {
            words.push(self.entity_name()?);
        }
        Ok(words.join(" "))
    }
}

fn is_cardinality(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::ZeroOrOne | Kind::ZeroOrMore | Kind::OneOrMore | Kind::OnlyOne | Kind::MdParent
    )
}

fn is_entity_start(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::EntityName | Kind::UnicodeText | Kind::Num | Kind::DecimalNum | Kind::EntityOne
    )
}
