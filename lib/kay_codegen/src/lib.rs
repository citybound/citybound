#[macro_use]
extern crate lazy_static;
extern crate regex;
use regex::Regex;

#[derive(Debug)]
struct MessageHandler {
    name: String,
    arguments: Vec<(String, String)>,
    returns_fate: bool,
    critical: bool,
}

impl MessageHandler {
    fn register(&self) -> String {
        let destructure_args = self.arguments.iter().map(|&(ref name, ref typ)|
            if typ.starts_with("&") {
                format!("ref {}", name)
            } else {
                name.to_owned()
            }
        ).collect::<Vec<_>>().join(", ");
        let params = self.arguments.iter().map(|&(ref name, _)|
            name.as_str()
        ).collect::<Vec<_>>().join(", ");
        format!(
"definer.on_critical(|&_KAY_MSG_{}({}), actor, world| {{
            actor.{}({}, world){}
        }});",
            self.name, destructure_args, self.name, params,
            if self.returns_fate {""} else {";\n            Fate::Live"}
        )
    }

    fn to_struct(&self) -> String {
        format!("#[allow(non_camel_case_types)]
#[derive(Compact, Clone)]
struct _KAY_MSG_{}({});", self.name, self.arguments.iter().map(|&(_, ref t)|
            t.as_str().trim_left_matches("&")
        ).collect::<Vec<_>>().join(", "))
    }

    fn to_id_func(&self) -> String {
        format!("pub fn {}(&self, {}, world: &mut World) {{
        world.send(self.0, _KAY_MSG_{}({}));
    }}",
            self.name,
            self.arguments.iter().map(|&(ref n, ref t)| format!("{}: {}", n, t.trim_left_matches("&"))).collect::<Vec<_>>().join(", "),
            self.name,
            self.arguments.iter().map(|&(ref n, _)| n.as_str()).collect::<Vec<_>>().join(", ")
            )
    }
}

#[derive(Debug)]
struct ActorDefinition {
    name: String,
    handlers: Vec<MessageHandler>,
}

impl ActorDefinition {
    fn to_setup(&self) -> String {
        format!(
"pub fn auto_setup(system: &mut ActorSystem, initial: {}) {{
    system.add(initial, |mut definer| {{
        {}
    }});
}}", self.name, self.handlers.iter().map(MessageHandler::register).collect::<Vec<_>>().join("\n\n        "))
    }

    fn structs(&self) -> String {
        self.handlers.iter().map(MessageHandler::to_struct).collect::<Vec<_>>().join("\n")
    }

    fn id(&self) -> String {
        format!(
"#[derive(Copy, Clone)]
pub struct {}ID(ID);

impl {}ID {{
    pub fn in_world(world: &mut World) -> {}ID {{
        {}ID(world.id::<{}>())
    }}

    {}
}}", self.name, self.name, self.name, self.name, self.name,
            self.handlers.iter().map(MessageHandler::to_id_func).collect::<Vec<_>>().join("\n    ")
        )
    }
}

pub fn generate(file: &str) -> String {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(?x)
            Define\sActor\s
            impl\s
               (?P<name>.+?)
            \s\{
                (?P<body>
                     (?:\s{4}[^\n]+?\n)*  # noncapturing group for indented lines
                )                   
            \}
            ").unwrap();
    }

    let generated_part = RE.captures_iter(file).map(|capt| {
        let def = read_actor_definition(&capt["name"], &capt["body"]);
        format!("{}\n\n{}\n\n{}", def.id(), def.structs(), def.to_setup())
    }).collect::<Vec<_>>().join("\n\n");

    format!("// ALL OF THIS IS AUTO-GENERATED, DON'T TOUCH
use kay::ActorSystem;
use super::*;\n{}", generated_part)
}

fn read_actor_definition(name: &str, body: &str) -> ActorDefinition {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(?x)
            Critical\s
            \s{4}pub\sfn\s(?P<name>.+?)\(&mut\sself,\s+
               (?P<args>(?:[\w:&\s<>]+?,)+)
            \s+(?:(?:world)|_):\s&mut\sWorld\)\s\{
                     (?:\s{8}[^\n]+?\n)*
            \s{4}\}
            ").unwrap();
    }
    ActorDefinition {
        name: name.to_owned(),
        handlers: RE.captures_iter(body).map(|capt|
            read_handler_definition(&capt["name"], &capt["args"])
        ).collect()
    }
}

fn read_handler_definition(name: &str, args: &str) -> MessageHandler {
    MessageHandler{
        name: name.to_owned(),
        arguments: args.split(",").filter_map(|pair|
            if pair.is_empty() {
                None
            } else {
                Some((
                    pair.split(":").nth(0).unwrap().trim().to_owned(),
                    pair.split(":").nth(1).unwrap().trim().to_owned(),
                ))
            }
        ).collect(),
        returns_fate: false,
        critical: true
    }
}