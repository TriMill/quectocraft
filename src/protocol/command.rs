use super::{data::PacketEncoder, clientbound::ClientBoundPacket};

#[derive(Debug, Clone)]
pub struct Commands {
    nodes: Vec<CommandNode>,
}

impl ClientBoundPacket for Commands {
    fn encode(&self, encoder: &mut impl PacketEncoder) {
        encoder.write_varint(self.nodes.len() as i32);
        for node in &self.nodes {
            node.encode(encoder);
        }
        // root node
        encoder.write_varint(0);
    }

    fn packet_id(&self) -> i32 { 0x0e }
}

impl Commands {
    pub fn new() -> Self {
        let root = CommandNode {
            executable: false,
            redirect: None,
            children: Vec::new(),
            suggestion: None,
            type_data: CommandNodeType::Root
        };
        let simple_cmd_arg = CommandNode {
            executable: true,
            redirect: None,
            children: Vec::new(),
            suggestion: None,
            type_data: CommandNodeType::Argument { name: "[args]".to_owned(), parser: Parser::String { kind: StringKind::Greedy } }
        };
        Self { 
            nodes: vec![root, simple_cmd_arg],
        }
    }

    pub fn create_node(
        &mut self, 
        parent: i32, 
        type_data: CommandNodeType, 
        executable: bool,
        redirect: Option<i32>, 
        suggestion: Option<String>
    ) -> Option<i32> {
        if parent < 0 || parent >= self.nodes.len() as i32 {
            return None
        }
        if let Some(redirect) = redirect {
            if redirect < 0 || redirect >= self.nodes.len() as i32 {
                return None
            }
        }
        if let CommandNodeType::Root = type_data {
            return None
        }
        let id = self.nodes.len() as i32;
        self.nodes.push(CommandNode {
            executable,
            redirect,
            children: Vec::new(),
            suggestion,
            type_data,
        });
        self.nodes[parent as usize].children.push(id);
        Some(id)
    }

    pub fn create_simple_cmd(&mut self, name: &str) -> Option<i32> {
        let id = self.create_node(0, CommandNodeType::Literal { name: name.to_owned() }, true, None, None)?;
        self.add_child(id, 1);
        Some(id)
    }

    pub fn add_child(&mut self, node: i32, child: i32) {
        self.nodes[node as usize].children.push(child);
    }

}

#[derive(Debug, Clone)]
pub struct CommandNode {
    executable: bool,
    redirect: Option<i32>,
    children: Vec<i32>,
    suggestion: Option<String>,
    type_data: CommandNodeType,
}

#[derive(Debug, Clone)]
pub enum CommandNodeType {
    Root, 
    Literal { name: String },
    Argument { name: String, parser: Parser }
}

impl CommandNode {
    pub fn encode(&self, encoder: &mut impl PacketEncoder) {
        let mut flags = match self.type_data {
            CommandNodeType::Root => 0,
            CommandNodeType::Literal{..} => 1,
            CommandNodeType::Argument{..} => 2,
        };
        if self.executable { flags |= 4; }
        if self.redirect.is_some() { flags |= 8; }
        if self.suggestion.is_some() { flags |= 16; }
        encoder.write_byte(flags);
        encoder.write_varint(self.children.len() as i32);
        for child in &self.children {
            encoder.write_varint(*child);
        }
        if let Some(redirect) = &self.redirect {
            encoder.write_varint(*redirect);
        }
        match &self.type_data {
            CommandNodeType::Root => (),
            CommandNodeType::Literal { name } => {
                encoder.write_string(32767, name);
            }
            CommandNodeType::Argument { name, parser } => {
                encoder.write_string(32767, name);
                parser.encode(encoder);
            }
        }
        if let Some(suggestion) = &self.suggestion {
            encoder.write_string(32767, suggestion);
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
pub enum Parser {
    Bool,
    Float { min: Option<f32>, max: Option<f32>, },    
    Double { min: Option<f64>, max: Option<f64>, },
    Int { min: Option<i32>, max: Option<i32>, },    
    Long { min: Option<i64>, max: Option<i64>, },
    String { kind: StringKind },
}

impl Parser {
    pub fn encode(&self, encoder: &mut impl PacketEncoder) {
        match self {
            Self::Bool => encoder.write_varint(0),
            Self::Float{ min, max } => {
                encoder.write_varint(1);
                encoder.write_byte( i8::from(min.is_some()) + 2 * i8::from(max.is_some()) );
                if let Some(min) = min { encoder.write_float(*min) };
                if let Some(max) = max { encoder.write_float(*max) };
            },
            Self::Double{ min, max } => {
                encoder.write_varint(2);
                encoder.write_byte( i8::from(min.is_some()) + 2 * i8::from(max.is_some()) );
                if let Some(min) = min { encoder.write_double(*min) };
                if let Some(max) = max { encoder.write_double(*max) };
            },
            Self::Int{ min, max } => {
                encoder.write_varint(3);
                encoder.write_byte( i8::from(min.is_some()) + 2 * i8::from(max.is_some()) );
                if let Some(min) = min { encoder.write_int(*min) };
                if let Some(max) = max { encoder.write_int(*max) };
            },
            Self::Long{ min, max } => {
                encoder.write_varint(4);
                encoder.write_byte( i8::from(min.is_some()) + 2 * i8::from(max.is_some()) );
                if let Some(min) = min { encoder.write_long(*min) };
                if let Some(max) = max { encoder.write_long(*max) };
            },
            Self::String{ kind } => {
                encoder.write_varint(5);
                encoder.write_varint(match kind {
                    StringKind::Single => 0,
                    StringKind::Quoted => 1,
                    StringKind::Greedy => 2,
                })
            },
        }        
    }
}

#[allow(unused)]
#[derive(Debug, Clone, Copy)]
pub enum StringKind {
    Single,
    Quoted,
    Greedy,
}
