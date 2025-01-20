use xml5ever::{
    serialize::{AttrRef, SerializeOpts, TraversalScope},
    QualName,
};

pub trait Node {
    /// # Errors
    /// If the underlying serialization fails
    fn serialize(&self) -> anyhow::Result<String>;

    /// # Errors
    /// If the underlying serialization fails
    fn serialize_with_options(&self, options: Options) -> anyhow::Result<String>;

    /// # Errors
    /// If the underlying serialization fails
    fn serialize_into<Wr: std::io::Write>(&self, sink: Wr) -> anyhow::Result<()>;
}

// WARN: Everything below is licensed from html5ever under the Apache License
struct NamespaceMapStack(Vec<xml5ever::tree_builder::NamespaceMap>);

impl NamespaceMapStack {
    fn new() -> NamespaceMapStack {
        NamespaceMapStack(vec![])
    }

    fn push(&mut self, namespace: xml5ever::tree_builder::NamespaceMap) {
        self.0.push(namespace);
    }

    fn pop(&mut self) {
        self.0.pop();
    }
}

pub struct Options {
    indent: usize,
    pretty: bool,
}

impl Options {
    pub fn new() -> Self {
        Self {
            indent: 4,
            pretty: false,
        }
    }
}

impl Default for Options {
    fn default() -> Self {
        Self::new()
    }
}

impl Options {
    pub fn pretty(mut self) -> Self {
        self.pretty = true;
        self
    }

    pub fn indent(mut self, indent: usize) -> Self {
        self.indent = indent;
        self
    }
}

#[derive(Default)]
pub struct State {
    indent_level: usize,
    text_content: Option<String>,
}

pub struct Serializer<Wr> {
    writer: Wr,
    namespace_stack: NamespaceMapStack,
    options: Options,
    state: State,
}

#[inline]
fn write_qual_name<W: std::io::Write>(writer: &mut W, name: &QualName) -> std::io::Result<()> {
    if let Some(ref prefix) = name.prefix {
        writer.write_all(prefix.as_bytes())?;
        writer.write_all(b":")?;
    }

    writer.write_all(name.local.as_bytes())?;
    Ok(())
}

impl<Wr: std::io::Write> Serializer<Wr> {
    pub fn new(writer: Wr) -> Self {
        Self {
            writer,
            namespace_stack: NamespaceMapStack::new(),
            options: Options::new(),
            state: State::default(),
        }
    }

    pub fn options(mut self, options: Options) -> Self {
        self.options = options;
        self
    }

    #[inline(always)]
    fn qual_name(&mut self, name: &QualName) -> std::io::Result<()> {
        self.find_or_insert_ns(name);
        write_qual_name(&mut self.writer, name)
    }

    #[inline(always)]
    fn qual_attr_name(&mut self, name: &QualName) -> std::io::Result<()> {
        self.find_or_insert_ns(name);
        write_qual_name(&mut self.writer, name)
    }

    fn find_uri(&self, name: &QualName) -> bool {
        let mut found = false;
        for stack in self.namespace_stack.0.iter().rev() {
            if let Some(Some(el)) = stack.get(&name.prefix) {
                found = *el == name.ns;
                break;
            }
        }
        found
    }

    fn find_or_insert_ns(&mut self, name: &QualName) {
        if (name.prefix.is_some() || !name.ns.is_empty()) && !self.find_uri(name) {
            if let Some(last_ns) = self.namespace_stack.0.last_mut() {
                last_ns.insert(name);
            }
        }
    }
}

// Custom alternative for markup5ever::Serializer

/// Writes given text into the Serializer, escaping it,
/// depending on where the text is written inside the tag or attribute value.
///
/// For example
///```text
///    <tag>'&-quotes'</tag>   becomes      <tag>'&amp;-quotes'</tag>
///    <tag = "'&-quotes'">    becomes      <tag = "&apos;&amp;-quotes&apos;"
///```
fn write_to_buf_escaped<W: std::io::Write>(
    writer: &mut W,
    text: &str,
    attr_mode: bool,
) -> std::io::Result<()> {
    for c in text.chars() {
        match c {
            '&' => writer.write_all(b"&amp;"),
            '\'' if attr_mode => writer.write_all(b"&apos;"),
            '"' if attr_mode => writer.write_all(b"&quot;"),
            '<' if !attr_mode => writer.write_all(b"&lt;"),
            '>' if !attr_mode => writer.write_all(b"&gt;"),
            c => writer.write_fmt(format_args!("{c}")),
        }?;
    }
    Ok(())
}

impl<Wr: std::io::Write> Serializer<Wr> {
    /// Serializes given start element into text. Start element contains
    /// qualified name and an attributes iterator.
    ///
    /// # Errors
    /// If the writer fails
    pub fn start_elem<'a, AttrIter>(
        &mut self,
        name: &QualName,
        attrs: AttrIter,
        is_empty: bool,
    ) -> std::io::Result<()>
    where
        AttrIter: Iterator<Item = AttrRef<'a>>,
    {
        self.namespace_stack
            .push(xml5ever::tree_builder::NamespaceMap::empty());

        self.create_indent()?;
        self.state.indent_level += 1;
        self.writer.write_all(b"<")?;
        self.qual_name(name)?;
        if let Some(current_namespace) = self.namespace_stack.0.last() {
            for (prefix, url_opt) in current_namespace.get_scope_iter() {
                self.writer.write_all(b" xmlns")?;
                if let Some(ref p) = *prefix {
                    self.writer.write_all(b":")?;
                    self.writer.write_all(p.as_bytes())?;
                }

                self.writer.write_all(b"=\"")?;
                let url = if let Some(ref a) = *url_opt {
                    a.as_bytes()
                } else {
                    b""
                };
                self.writer.write_all(url)?;
                self.writer.write_all(b"\"")?;
            }
        }
        for (name, value) in attrs {
            self.writer.write_all(b" ")?;
            self.qual_attr_name(name)?;
            self.writer.write_all(b"=\"")?;
            write_to_buf_escaped(&mut self.writer, value, true)?;
            self.writer.write_all(b"\"")?;
        }
        if is_empty {
            self.state.indent_level -= 1;
            self.writer.write_all(b"/")?;
        }
        self.writer.write_all(b">")?;
        Ok(())
    }

    /// Serializes given end element into text.
    ///
    /// # Errors
    /// If the writer fails
    pub fn end_elem(&mut self, name: &QualName) -> std::io::Result<()> {
        self.namespace_stack.pop();
        if self.state.indent_level > 0 {
            self.state.indent_level -= 1;
        }
        self.create_indent()?;
        self.writer.write_all(b"</")?;
        self.qual_name(name)?;
        self.writer.write_all(b">")
    }

    /// Serializes comment into text.
    ///
    /// # Errors
    /// If the writer fails
    pub fn write_comment(&mut self, text: &str) -> std::io::Result<()> {
        self.create_indent()?;
        self.writer.write_all(b"<!--")?;
        self.writer.write_all(text.as_bytes())?;
        self.writer.write_all(b"-->")
    }

    /// Serializes given doctype
    ///
    /// # Errors
    /// If the writer fails
    pub fn write_doctype(&mut self, name: &str) -> std::io::Result<()> {
        self.create_indent()?;
        self.writer.write_all(b"<!DOCTYPE ")?;
        self.writer.write_all(name.as_bytes())?;
        self.writer.write_all(b">")
    }

    /// Serializes text for a node or an attributes.
    ///
    /// # Errors
    /// If the writer fails
    pub fn write_text(&mut self, text: &str) -> std::io::Result<()> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(());
        }
        self.create_indent()?;
        write_to_buf_escaped(&mut self.writer, text.trim(), false)
    }

    /// Serializes given processing instruction.
    ///
    /// # Errors
    /// If the writer fails
    pub fn write_processing_instruction(
        &mut self,
        target: &str,
        data: &str,
    ) -> std::io::Result<()> {
        self.writer.write_all(b"<?")?;
        self.writer.write_all(target.as_bytes())?;
        self.writer.write_all(b" ")?;
        self.writer.write_all(data.as_bytes())?;
        self.writer.write_all(b"?>")
    }

    fn create_indent(&mut self) -> std::io::Result<()> {
        if self.options.pretty && self.state.text_content.is_none() {
            self.writer.write_all(b"\n")?;
            let indent = b" ".repeat(self.options.indent * self.state.indent_level);
            self.writer.write_all(&indent)
        } else {
            Ok(())
        }
    }
}

/// Types that can be serialized (according to the xml-like scheme in `Serializer`) implement this
/// trait.
pub trait Serialize {
    /// Take the serializer and call its methods to serialize this type. The type will dictate
    /// which methods are called and with what parameters.
    ///
    /// # Errors
    /// If the writer fails
    fn serialize<Wr: std::io::Write>(
        &self,
        serializer: &mut Serializer<Wr>,
        traversal_scope: TraversalScope,
    ) -> std::io::Result<()>;
}

/// Method for serializing generic node to a given writer.
///
/// # Errors
/// If the writer fails
pub fn serialize<Wr, T>(writer: Wr, node: &T, opts: SerializeOpts) -> std::io::Result<()>
where
    Wr: std::io::Write,
    T: Serialize,
{
    let mut ser = Serializer::new(writer);
    node.serialize(&mut ser, opts.traversal_scope)
}

/// # Errors
/// If the writer fails
pub fn with_options<Wr, T>(
    writer: Wr,
    node: &T,
    opts: SerializeOpts,
    options: Options,
) -> std::io::Result<()>
where
    Wr: std::io::Write,
    T: Serialize,
{
    let mut ser = Serializer::new(writer).options(options);
    node.serialize(&mut ser, opts.traversal_scope)
}
