use cosmic::{
    iced_core::{Alignment, Length, Padding},
    iced_widget::{
        scrollable::{Direction, Properties},
        PickList, Scrollable, Toggler,
    },
    style,
    widget::{Column, Container, Row, Slider, Space, Text, TextInput},
    Element,
};
use data::{
    app_graph::Nodes,
    config::custom_temp::CustomTempKind,
    node::{Node, NodeType, NodeTypeLight, ValueKind},
};
use hardware::Hardware;

use crate::{
    input_line::{input_line, InputLineUnit},
    pick::{pick_hardware, pick_input, Pick},
    utils::{icon_button, icon_path_for_node_type, my_icon},
    AppMsg, ModifNodeMsg,
};

pub fn items_view<'a>(nodes: &'a Nodes, hardware: &'a Hardware) -> Element<'a, AppMsg> {
    let mut controls = Vec::new();
    let mut behaviors = Vec::new();
    let mut temps = Vec::new();
    let mut fans = Vec::new();

    for node in nodes.values() {
        match node.node_type.to_light() {
            NodeTypeLight::Control => controls.push(control_view(node, nodes, hardware)),
            NodeTypeLight::Fan => fans.push(fan_view(node, hardware)),
            NodeTypeLight::Temp => temps.push(temp_view(node, hardware)),
            NodeTypeLight::CustomTemp => temps.push(custom_temp_view(node, nodes)),
            NodeTypeLight::Graph => {}
            NodeTypeLight::Flat => behaviors.push(flat_view(node)),
            NodeTypeLight::Linear => behaviors.push(linear_view(node, nodes)),
            NodeTypeLight::Target => behaviors.push(target_view(node, nodes)),
        }
    }

    let list_views = vec![
        list_view(controls),
        list_view(behaviors),
        list_view(temps),
        list_view(fans),
    ];

    let content = Row::with_children(list_views).spacing(20).padding(25);

    let container = Container::new(content);

    Scrollable::new(container)
        .direction(Direction::Both {
            vertical: Properties::default(),
            horizontal: Properties::default(),
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn list_view(elements: Vec<Element<AppMsg>>) -> Element<AppMsg> {
    Column::with_children(elements)
        .spacing(20)
        .padding(25)
        .into()
}

fn item_view<'a>(node: &'a Node, content: Vec<Element<'a, ModifNodeMsg>>) -> Element<'a, AppMsg> {
    let item_icon = my_icon(icon_path_for_node_type(&node.node_type.to_light()));

    let mut name = TextInput::new("name", &node.name_cached)
        .on_input(|s| AppMsg::ModifNode(node.id, ModifNodeMsg::Rename(s)));

    if node.is_error_name {
        name = name.error("this name is already beeing use");
    }

    // todo: dropdown menu
    let delete_button =
        icon_button("select/delete_forever24").on_press(AppMsg::DeleteNode(node.id));

    let top = Row::new()
        .push(item_icon)
        .push(name)
        .push(delete_button)
        .align_items(Alignment::Center);

    let content: Element<ModifNodeMsg> = Column::with_children(content).spacing(5).into();

    let column: Element<AppMsg> = Column::new()
        .push(top)
        .push(content.map(|m| AppMsg::ModifNode(node.id, m)))
        .into();

    Container::new(column)
        .width(Length::Fixed(200.0))
        .padding(Padding::new(10.0))
        .style(style::Container::Card)
        .into()
}

#[derive(Debug, Clone)]
pub enum ControlMsg {
    Active(bool),
}

fn control_view<'a>(
    node: &'a Node,
    nodes: &'a Nodes,
    hardware: &'a Hardware,
) -> Element<'a, AppMsg> {
    let NodeType::Control(control) = &node.node_type else {
        panic!()
    };

    let content = vec![
        pick_hardware(node, &hardware.controls, true),
        pick_input(
            node,
            nodes,
            &control.input,
            true,
            Box::new(ModifNodeMsg::ReplaceInput),
        ),
        Row::new()
            .push(Text::new(node.value_text(&ValueKind::Porcentage)))
            .push(Space::new(Length::Fill, Length::Fixed(0.0)))
            .push(Toggler::new(None, control.active, |is_active| {
                ModifNodeMsg::Control(ControlMsg::Active(is_active))
            }))
            .align_items(Alignment::Center)
            .width(Length::Fill)
            .into(),
    ];

    item_view(node, content)
}

fn temp_view<'a>(node: &'a Node, hardware: &'a Hardware) -> Element<'a, AppMsg> {
    let content = vec![
        pick_hardware(node, &hardware.temps, false),
        Text::new(node.value_text(&ValueKind::Celsius)).into(),
    ];

    item_view(node, content)
}

fn fan_view<'a>(node: &'a Node, hardware: &'a Hardware) -> Element<'a, AppMsg> {
    let content = vec![
        pick_hardware(node, &hardware.fans, false),
        Text::new(node.value_text(&ValueKind::RPM)).into(),
    ];

    item_view(node, content)
}

#[derive(Debug, Clone)]
pub enum CustomTempMsg {
    Kind(CustomTempKind),
}

fn custom_temp_view<'a>(node: &'a Node, nodes: &'a Nodes) -> Element<'a, AppMsg> {
    let NodeType::CustomTemp(custom_temp) = &node.node_type else {
        panic!()
    };

    let inputs = node
        .inputs
        .iter()
        .map(|i| {
            Row::new()
                .push(Text::new(i.1.clone()))
                .push(Space::new(Length::Fill, Length::Fixed(0.0)))
                .push(
                    icon_button("select/close/close20")
                        .on_press(ModifNodeMsg::RemoveInput(Pick::new(&i.1, &i.0))),
                )
                .align_items(Alignment::Center)
                .into()
        })
        .collect();

    let kind_options = CustomTempKind::VALUES
        .iter()
        .filter(|k| &custom_temp.kind != *k)
        .cloned()
        .collect::<Vec<_>>();

    let pick_kind = PickList::new(kind_options, Some(custom_temp.kind.clone()), |k| {
        ModifNodeMsg::CustomTemp(CustomTempMsg::Kind(k))
    })
    .width(Length::Fill)
    .into();
    let content = vec![
        pick_kind,
        pick_input(
            node,
            nodes,
            &Some("Choose Temp".into()),
            false,
            Box::new(ModifNodeMsg::AddInput),
        ),
        Column::with_children(inputs).into(),
        Text::new(node.value_text(&ValueKind::Celsius)).into(),
    ];

    item_view(node, content)
}

#[derive(Debug, Clone)]
pub enum FlatMsg {
    Value(u16),
}

fn flat_view(node: &Node) -> Element<AppMsg> {
    let NodeType::Flat(flat) = &node.node_type else {
        panic!()
    };

    let mut sub_button = icon_button("sign/minus/remove24");
    if flat.value > 0 {
        sub_button = sub_button.on_press(ModifNodeMsg::Flat(FlatMsg::Value(flat.value - 1)));
    }

    let mut add_button = icon_button("sign/plus/add24");
    if flat.value < 100 {
        add_button = add_button.on_press(ModifNodeMsg::Flat(FlatMsg::Value(flat.value + 1)));
    }

    let buttons = Row::new()
        .push(sub_button)
        .push(add_button)
        .align_items(Alignment::Center);

    let buttons = Row::new()
        .push(Text::new(node.value_text(&ValueKind::Porcentage)))
        .push(Space::new(Length::Fill, Length::Fixed(0.0)))
        .push(buttons)
        .align_items(Alignment::Center)
        .into();

    let slider = Slider::new(0..=100, flat.value, |v| {
        ModifNodeMsg::Flat(FlatMsg::Value(v))
    })
    .into();

    let content = vec![buttons, slider];

    item_view(node, content)
}

#[derive(Debug, Clone)]
pub enum LinearMsg {
    MinTemp(u8, String),
    MinSpeed(u8, String),
    MaxTemp(u8, String),
    MaxSpeed(u8, String),
}

fn linear_view<'a>(node: &'a Node, nodes: &'a Nodes) -> Element<'a, AppMsg> {
    let NodeType::Linear(linear, linear_cache) = &node.node_type else {
        panic!()
    };

    let content = vec![
        pick_input(
            node,
            nodes,
            &linear.input,
            true,
            Box::new(ModifNodeMsg::ReplaceInput),
        ),
        Text::new(node.value_text(&ValueKind::Porcentage)).into(),
        input_line(
            "min temp",
            &linear.min_temp,
            &linear_cache.min_temp,
            InputLineUnit::Celcius,
            &(0..=255),
            |val, cached_val| ModifNodeMsg::Linear(LinearMsg::MinTemp(val, cached_val)),
        ),
        input_line(
            "min speed",
            &linear.min_speed,
            &linear_cache.min_speed,
            InputLineUnit::Porcentage,
            &(0..=100),
            |val, cached_val| ModifNodeMsg::Linear(LinearMsg::MinSpeed(val, cached_val)),
        ),
        input_line(
            "max temp",
            &linear.max_temp,
            &linear_cache.max_temp,
            InputLineUnit::Celcius,
            &(0..=255),
            |val, cached_val| ModifNodeMsg::Linear(LinearMsg::MaxTemp(val, cached_val)),
        ),
        input_line(
            "max speed",
            &linear.max_speed,
            &linear_cache.max_speed,
            InputLineUnit::Porcentage,
            &(0..=100),
            |val, cached_val| ModifNodeMsg::Linear(LinearMsg::MaxSpeed(val, cached_val)),
        ),
    ];

    item_view(node, content)
}

#[derive(Debug, Clone)]
pub enum TargetMsg {
    IdleTemp(u8, String),
    IdleSpeed(u8, String),
    LoadTemp(u8, String),
    LoadSpeed(u8, String),
}

fn target_view<'a>(node: &'a Node, nodes: &'a Nodes) -> Element<'a, AppMsg> {
    let NodeType::Target(target, target_cache) = &node.node_type else {
        panic!()
    };

    let content = vec![
        pick_input(
            node,
            nodes,
            &target.input,
            true,
            Box::new(ModifNodeMsg::ReplaceInput),
        ),
        Text::new(node.value_text(&ValueKind::Porcentage)).into(),
        input_line(
            "idle temp",
            &target.idle_temp,
            &target_cache.idle_temp,
            InputLineUnit::Celcius,
            &(0..=255),
            |val, cached_val| ModifNodeMsg::Target(TargetMsg::IdleTemp(val, cached_val)),
        ),
        input_line(
            "idle speed",
            &target.idle_speed,
            &target_cache.idle_speed,
            InputLineUnit::Porcentage,
            &(0..=100),
            |val, cached_val| ModifNodeMsg::Target(TargetMsg::IdleSpeed(val, cached_val)),
        ),
        input_line(
            "load temp",
            &target.load_temp,
            &target_cache.load_temp,
            InputLineUnit::Celcius,
            &(0..=255),
            |val, cached_val| ModifNodeMsg::Target(TargetMsg::LoadTemp(val, cached_val)),
        ),
        input_line(
            "load speed",
            &target.load_speed,
            &target_cache.load_speed,
            InputLineUnit::Porcentage,
            &(0..=100),
            |val, cached_val| ModifNodeMsg::Target(TargetMsg::LoadSpeed(val, cached_val)),
        ),
    ];

    item_view(node, content)
}
