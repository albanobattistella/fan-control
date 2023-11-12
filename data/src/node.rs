use std::collections::HashMap;

use hardware::{Hardware, Value};
use light_enum::LightEnum;

use crate::config::linear::LinearCache;
use crate::config::Config;
use crate::config::{
    control::Control, custom_temp::CustomTemp, fan::Fan, flat::Flat, graph::Graph, linear::Linear,
    target::Target, temp::Temp,
};

use crate::id::{Id, IdGenerator};

pub type Nodes = HashMap<Id, Node>;
pub type RootNodes = Vec<Id>;

#[derive(Debug)]
pub struct AppGraph {
    pub nodes: Nodes,
    pub id_generator: IdGenerator,
    pub root_nodes: RootNodes,
}

impl AppGraph {
    fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            id_generator: IdGenerator::new(),
            root_nodes: Vec::new(),
        }
    }

    pub fn default(hardware: &Hardware) -> Self {
        let mut app_graph = AppGraph::new();

        for control_h in &hardware.controls {
            let control = Control::new(
                control_h.name.clone(),
                Some(control_h.hardware_id.clone()),
                None,
                true,
                Some(control_h.clone()),
            );

            let node = Node::new(
                &mut app_graph.id_generator,
                NodeType::Control(control),
                Vec::new(),
            );
            app_graph.root_nodes.push(node.id);
            app_graph.nodes.insert(node.id, node);
        }

        for fan_h in &hardware.fans {
            let fan = Fan {
                name: fan_h.name.clone(),
                hardware_id: Some(fan_h.hardware_id.clone()),
                fan_h: Some(fan_h.clone()),
            };

            let node = Node::new(&mut app_graph.id_generator, NodeType::Fan(fan), Vec::new());
            app_graph.nodes.insert(node.id, node);
        }

        for temp_h in &hardware.temps {
            let temp = Temp {
                name: temp_h.name.clone(),
                hardware_id: Some(temp_h.hardware_id.clone()),
                temp_h: Some(temp_h.clone()),
            };

            let node = Node::new(
                &mut app_graph.id_generator,
                NodeType::Temp(temp),
                Vec::new(),
            );
            app_graph.nodes.insert(node.id, node);
        }

        app_graph
    }

    pub fn from_config(config: Config, hardware: &Hardware) -> Self {
        let mut app_graph = AppGraph::new();

        // order: fan -> temp -> custom_temp -> behavior -> control

        for fan in config.fans {
            let node = fan.to_node(&mut app_graph.id_generator, &app_graph.nodes, hardware);
            app_graph.nodes.insert(node.id, node);
        }

        for temp in config.temps {
            let node = temp.to_node(&mut app_graph.id_generator, &app_graph.nodes, hardware);
            app_graph.nodes.insert(node.id, node);
        }

        for custom_temp in config.custom_temps {
            let node = custom_temp.to_node(&mut app_graph.id_generator, &app_graph.nodes, hardware);
            app_graph.nodes.insert(node.id, node);
        }

        for flat in config.flats {
            let node = flat.to_node(&mut app_graph.id_generator, &app_graph.nodes, hardware);
            app_graph.nodes.insert(node.id, node);
        }

        for linear in config.linears {
            let node = linear.to_node(&mut app_graph.id_generator, &app_graph.nodes, hardware);
            app_graph.nodes.insert(node.id, node);
        }

        for target in config.targets {
            let node = target.to_node(&mut app_graph.id_generator, &app_graph.nodes, hardware);
            app_graph.nodes.insert(node.id, node);
        }

        for control in config.controls {
            let node = control.to_node(&mut app_graph.id_generator, &app_graph.nodes, hardware);
            app_graph.root_nodes.push(node.id);
            app_graph.nodes.insert(node.id, node);
        }

        app_graph
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: Id,
    pub node_type: NodeType,
    pub inputs: Vec<(Id, String)>,

    pub value: Option<Value>,

    pub name_cached: String,
    pub is_error_name: bool,
}

#[derive(Debug, Clone, LightEnum)]
pub enum NodeType {
    Control(Control),
    Fan(Fan),
    Temp(Temp),
    CustomTemp(CustomTemp),
    Graph(Graph),
    Flat(Flat),
    // todo: i think enum support 2 field, but not light_enum
    Linear((Linear, LinearCache)),
    Target(Target),
}

impl NodeType {
    pub fn name(&self) -> &String {
        match self {
            NodeType::Control(control) => &control.name,
            NodeType::Fan(fan) => &fan.name,
            NodeType::Temp(temp) => &temp.name,
            NodeType::CustomTemp(custom_temp) => &custom_temp.name,
            NodeType::Graph(graph) => &graph.name,
            NodeType::Flat(flat) => &flat.name,
            NodeType::Linear((linear, _)) => &linear.name,
            NodeType::Target(target) => &target.name,
        }
    }
}

impl Node {
    pub fn new(
        id_generator: &mut IdGenerator,
        node_type: NodeType,
        inputs: Vec<(Id, String)>,
    ) -> Self {
        let name_cached = node_type.name().clone();
        Self {
            id: id_generator.new_id(),
            node_type,
            inputs,
            value: None,
            name_cached,
            is_error_name: false,
        }
    }

    pub fn name(&self) -> &String {
        self.node_type.name()
    }

    #[allow(clippy::result_unit_err)]
    pub fn hardware_id(&self) -> Result<&Option<String>, ()> {
        match &self.node_type {
            NodeType::Control(i) => Ok(&i.hardware_id),
            NodeType::Fan(i) => Ok(&i.hardware_id),
            NodeType::Temp(i) => Ok(&i.hardware_id),
            _ => Err(()),
        }
    }
}

impl IsValid for Node {
    fn is_valid(&self) -> bool {
        match &self.node_type {
            NodeType::Control(control) => control.is_valid(),
            NodeType::Fan(fan) => fan.is_valid(),
            NodeType::Temp(temp) => temp.is_valid(),
            NodeType::CustomTemp(custom_temp) => custom_temp.is_valid(),
            NodeType::Graph(graph) => graph.is_valid(),
            NodeType::Flat(flat) => flat.is_valid(),
            NodeType::Linear((linear, _)) => linear.is_valid(),
            NodeType::Target(target) => target.is_valid(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NbInput {
    Zero,
    One,
    Infinity,
}

impl NodeTypeLight {
    pub fn allowed_dep(&self) -> &'static [NodeTypeLight] {
        match self {
            NodeTypeLight::Control => &[
                NodeTypeLight::Flat,
                NodeTypeLight::Graph,
                NodeTypeLight::Target,
                NodeTypeLight::Linear,
            ],
            NodeTypeLight::Fan => &[],
            NodeTypeLight::Temp => &[],
            NodeTypeLight::CustomTemp => &[NodeTypeLight::Temp],
            NodeTypeLight::Graph => &[NodeTypeLight::Temp, NodeTypeLight::CustomTemp],
            NodeTypeLight::Flat => &[],
            NodeTypeLight::Linear => &[NodeTypeLight::Temp, NodeTypeLight::CustomTemp],
            NodeTypeLight::Target => &[NodeTypeLight::Temp, NodeTypeLight::CustomTemp],
        }
    }

    pub fn max_input(&self) -> NbInput {
        match self {
            NodeTypeLight::Control => NbInput::One,
            NodeTypeLight::Fan => NbInput::Zero,
            NodeTypeLight::Temp => NbInput::Zero,
            NodeTypeLight::CustomTemp => NbInput::Infinity,
            NodeTypeLight::Graph => NbInput::One,
            NodeTypeLight::Flat => NbInput::Zero,
            NodeTypeLight::Linear => NbInput::One,
            NodeTypeLight::Target => NbInput::One,
        }
    }
}

pub trait IsValid {
    fn is_valid(&self) -> bool;
}

pub trait ToNode {
    fn to_node(self, id_generator: &mut IdGenerator, nodes: &Nodes, hardware: &Hardware) -> Node;
}

pub trait Inputs {
    fn clear_inputs(&mut self);
    fn get_inputs(&self) -> Vec<&String>;
}

pub fn sanitize_inputs(
    item: &mut impl Inputs,
    nodes: &Nodes,
    node_type: NodeTypeLight,
) -> Vec<(Id, String)> {
    let mut inputs = Vec::new();

    match node_type.max_input() {
        NbInput::Zero => {
            if !item.get_inputs().is_empty() {
                eprintln!(
                    "{:?}: number of dep allowed == {:?}",
                    node_type,
                    node_type.max_input()
                );
                item.clear_inputs();
            };
            return inputs;
        }
        NbInput::One => {
            if item.get_inputs().len() > 1 {
                eprintln!(
                    "{:?}: number of dep allowed == {:?}",
                    node_type,
                    node_type.max_input()
                );
                item.clear_inputs();
                return inputs;
            }
        }
        _ => {}
    };

    for name in item.get_inputs() {
        if let Some(node) = nodes.values().find(|node| node.name() == name) {
            if !node_type.allowed_dep().contains(&node.node_type.to_light()) {
                eprintln!(
                    "sanitize_inputs: incompatible node type. {:?} <- {}. Fall back: remove all",
                    node.node_type.to_light(),
                    name
                );
                item.clear_inputs();
                inputs.clear();
                return inputs;
            }
            inputs.push((node.id, name.clone()))
        } else {
            eprintln!(
                "sanitize_inputs: can't find {} in app_graph. Fall back: remove all",
                name
            );
            item.clear_inputs();
            inputs.clear();
            return inputs;
        }
    }

    if node_type.max_input() == NbInput::One && inputs.len() > 1 {
        item.clear_inputs();
        inputs.clear();
        return inputs;
    }
    inputs
}

pub fn validate_name(nodes: &Nodes, id: &Id, name: &String) -> bool {
    if name.is_empty() {
        return false;
    };

    for node in nodes.values() {
        if node.name() == name && &node.id != id {
            return false;
        }
    }

    true
}
