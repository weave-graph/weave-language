//! Core contract v1 classification of emitted plans and artifacts.
//!
//! The engine's `docs/contract/core-v1.md` section 12 classifies every contract
//! feature from 0.1 to 0.22 as *core v1*, *extension* or *deferred*. This module
//! walks what the compiler actually emitted (commands, graph expressions, the
//! data-level extension fields and host templates) and reports each feature
//! with that class and the exact JSON locations that use it.
//!
//! The walk matches every `Command` and `GraphExpression` variant without a
//! wildcard, so a vendor update that adds a variant fails to build until the
//! variant is classified. The classification never changes what is emitted.
use std::collections::BTreeMap;
use weave_contract::ScalarType;
use weave_contract::facts::{Fact, FactChange};
use weave_contract::{Command, GraphData, GraphExpression, MetadataAttachment, Node, Program};

/// Profile name of the engine's consolidated core specification.
pub const CORE_PROFILE: &str = weave_contract::wire::CORE_PROFILE;

/// A section 12 class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum FeatureClass {
    /// Required, specified by core-v1.md and built on by engine Phases A-C.
    #[serde(rename = "core-v1")]
    CoreV1,
    /// Supported and tested, but optional; governed by its own version note.
    #[serde(rename = "extension")]
    Extension,
    /// Belongs to an epic the engine roadmap defers; maintained, not developed.
    #[serde(rename = "deferred")]
    Deferred,
}

/// One section 12 feature used by the compiled output.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct FeatureUse {
    /// Stable feature name: the wire name of the command, expression, field or artifact.
    pub feature: &'static str,
    /// What the name identifies: a command `op`, an expression `kind`, a data
    /// `field` or a host `artifact`.
    pub construct: &'static str,
    /// Contract version that introduced the feature.
    pub since: &'static str,
    pub class: FeatureClass,
    /// JSON-pointer-style locations in the emitted artifacts output
    /// (`/program/commands/3/value/input`, `/view_templates/Active`).
    pub locations: Vec<String>,
}

/// Classification of one compiled source against core contract v1.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ContractProfile {
    /// `Program.version` of the emitted plan.
    pub protocol: String,
    /// Always [`CORE_PROFILE`].
    pub profile: &'static str,
    /// True when every emitted feature is classified core v1.
    pub core_only: bool,
    /// Every emitted feature, ordered by class (core, extension, deferred), name and construct.
    pub features: Vec<FeatureUse>,
}

#[derive(Default)]
struct Collector {
    uses: BTreeMap<(FeatureClass, &'static str, &'static str), (&'static str, Vec<String>)>,
    construct: &'static str,
}

impl Collector {
    fn add(&mut self, feature: &'static str, since: &'static str, class: FeatureClass, at: &str) {
        self.uses
            .entry((class, feature, self.construct))
            .or_insert_with(|| (since, Vec::new()))
            .1
            .push(at.to_owned());
    }

    fn command(&mut self, command: &Command, at: &str) {
        use FeatureClass::*;
        self.construct = "command";
        match command {
            Command::Commit { data, .. } => {
                self.add("commit", "0.1", CoreV1, at);
                self.data(data, &format!("{at}/data"));
            }
            Command::CommitBatch { commits, .. } => {
                self.add("commit_batch", "0.4", CoreV1, at);
                for (index, commit) in commits.iter().enumerate() {
                    self.data(&commit.data, &format!("{at}/commits/{index}/data"));
                }
            }
            Command::Query { .. } => self.add("query", "0.1", CoreV1, at),
            Command::Join { .. } => self.add("join", "0.2", CoreV1, at),
            Command::Bind { value, .. } => {
                self.add("bind", "0.3", CoreV1, at);
                self.expression(value, &format!("{at}/value"));
            }
            Command::Evaluate { value } => {
                self.add("evaluate", "0.3", CoreV1, at);
                self.expression(value, &format!("{at}/value"));
            }
            Command::RecordedRange { .. } => self.add("recorded_range", "0.21", CoreV1, at),
            Command::AcceptedRange { .. } => self.add("accepted_range", "0.21", Extension, at),
            Command::Write { changes, .. } => {
                self.add("write", "0.22", CoreV1, at);
                for (index, change) in changes.iter().enumerate() {
                    let fact = match change {
                        FactChange::Assert { fact } | FactChange::Correct { fact, .. } => fact,
                        FactChange::Retract { .. } => continue,
                    };
                    self.fact(fact, &format!("{at}/changes/{index}/fact"));
                }
            }
        }
    }

    fn expression(&mut self, expression: &GraphExpression, at: &str) {
        use FeatureClass::*;
        let mut inputs: Vec<(&GraphExpression, String)> = Vec::new();
        let (feature, since, class) = match expression {
            GraphExpression::Query { .. } => ("query", "0.3", CoreV1),
            GraphExpression::Reference { .. } => ("reference", "0.3", CoreV1),
            GraphExpression::Filter { input, .. } => {
                inputs.push((input, "input".into()));
                ("filter", "0.3", CoreV1)
            }
            GraphExpression::Join { left, right, .. } => {
                inputs.push((left, "left".into()));
                inputs.push((right, "right".into()));
                ("join", "0.3", CoreV1)
            }
            GraphExpression::Metadata { input, .. } => {
                inputs.push((input, "input".into()));
                ("metadata", "0.4", CoreV1)
            }
            GraphExpression::Union { left, right } => {
                inputs.push((left, "left".into()));
                inputs.push((right, "right".into()));
                ("union", "0.5", CoreV1)
            }
            GraphExpression::Diff { before, after } => {
                inputs.push((before, "before".into()));
                inputs.push((after, "after".into()));
                ("diff", "0.5", CoreV1)
            }
            GraphExpression::Project { input, .. } => {
                inputs.push((input, "input".into()));
                ("project", "0.5", CoreV1)
            }
            GraphExpression::Support { input, .. } => {
                inputs.push((input, "input".into()));
                ("support", "0.5", CoreV1)
            }
            GraphExpression::Reason { input, .. } => {
                inputs.push((input, "input".into()));
                ("reason", "0.7", Extension)
            }
            GraphExpression::Context { input, .. } => {
                inputs.push((input, "input".into()));
                ("context", "0.8", Extension)
            }
            GraphExpression::Explain { input } => {
                inputs.push((input, "input".into()));
                ("explain", "0.9", CoreV1)
            }
            GraphExpression::Geometry { operation, .. } => {
                use weave_contract::GeometryOperation as G;
                match operation {
                    G::Distance { left, right } => {
                        inputs.push((&left.input, "operation/left/input".into()));
                        inputs.push((&right.input, "operation/right/input".into()));
                    }
                    G::Transform { input, mapping } => {
                        inputs.push((&input.input, "operation/input/input".into()));
                        inputs.push((&mapping.input, "operation/mapping/input".into()));
                    }
                    G::ProjectAxes { input, .. } => {
                        inputs.push((&input.input, "operation/input/input".into()));
                    }
                }
                ("geometry", "0.9", Extension)
            }
            GraphExpression::Counterparts { input, .. } => {
                inputs.push((input, "input".into()));
                ("counterparts", "0.10", Extension)
            }
            GraphExpression::ResolveIdentity { .. } => ("resolve_identity", "0.13", Extension),
            GraphExpression::Cluster { .. } => ("cluster", "0.13", Deferred),
            GraphExpression::TypedContext { input, .. } => {
                inputs.push((input, "input".into()));
                ("typed_context", "0.14", Extension)
            }
            GraphExpression::AcceptedGraph { .. } => ("accepted_graph", "0.16", Extension),
            GraphExpression::CurrentView { .. } => ("current_view", "0.16", Extension),
            GraphExpression::Window { input, .. } => {
                inputs.push((input, "input".into()));
                ("window", "0.19", CoreV1)
            }
            GraphExpression::Sequence { left, right, .. } => {
                inputs.push((left, "left".into()));
                inputs.push((right, "right".into()));
                ("sequence", "0.19", CoreV1)
            }
            GraphExpression::RecordedQuery { .. } => ("recorded_query", "0.20", CoreV1),
            GraphExpression::AcceptedHistory { .. } => ("accepted_history", "0.21", Extension),
        };
        self.construct = "expression";
        self.add(feature, since, class, at);
        for (input, field) in inputs {
            self.expression(input, &format!("{at}/{field}"));
        }
    }

    /// Extension fields of one fact record carried by a `write` change. Node
    /// `valid_time` and claimed `provenance` (0.22) are core v1.
    fn fact(&mut self, fact: &Fact, at: &str) {
        self.construct = "field";
        match fact {
            Fact::Node(node) => self.node(node, at),
            Fact::Attachment(attachment) => self.attachment(attachment, at),
            Fact::Edge(_) | Fact::StructuralEdge(_) | Fact::Assertion(_) => {}
        }
    }

    fn node(&mut self, node: &Node, at: &str) {
        if node.context_scope.is_some() {
            self.add(
                "context_scope",
                "0.8",
                FeatureClass::Extension,
                &format!("{at}/context_scope"),
            );
        }
    }

    fn attachment(&mut self, attachment: &MetadataAttachment, at: &str) {
        if attachment.context.is_some() {
            self.add(
                "attachment_context",
                "0.8",
                FeatureClass::Extension,
                &format!("{at}/context"),
            );
        }
    }

    /// Data-level fields that section 12 classifies as extensions. Every other
    /// `GraphData` field emitted by this compiler is core v1.
    fn data(&mut self, data: &GraphData, at: &str) {
        use FeatureClass::*;
        self.construct = "field";
        if data.context_typing.is_some() {
            self.add(
                "context_typing",
                "0.14",
                Extension,
                &format!("{at}/context_typing"),
            );
        }
        for (index, node) in data.nodes.iter().enumerate() {
            self.node(node, &format!("{at}/nodes/{index}"));
        }
        for (index, attachment) in data.attachments.iter().enumerate() {
            self.attachment(attachment, &format!("{at}/attachments/{index}"));
        }
        if let Some(schema) = &data.schema {
            let nodes = schema
                .nodes
                .iter()
                .map(|(name, node)| (format!("nodes/{name}"), &node.properties));
            let edges = schema
                .edges
                .iter()
                .map(|(name, edge)| (format!("edges/{name}"), &edge.properties));
            for (owner, properties) in nodes.chain(edges) {
                for (property, declaration) in properties {
                    let feature = match declaration.value_type {
                        ScalarType::Decimal => "decimal",
                        ScalarType::Quantity(_) => "quantity",
                        ScalarType::Float
                        | ScalarType::String
                        | ScalarType::Integer
                        | ScalarType::Boolean => continue,
                    };
                    self.add(
                        feature,
                        "0.12",
                        Extension,
                        &format!("{at}/schema/{owner}/properties/{property}"),
                    );
                }
            }
        }
    }
}

/// Classify a plan and its host templates, as emitted by `compile_artifacts`.
pub fn classify(
    program: &Program,
    view_templates: &BTreeMap<String, weave_contract::CompiledViewTemplate>,
    handler_templates: &BTreeMap<String, weave_contract::CompiledHandlerTemplate>,
) -> ContractProfile {
    let mut collector = Collector::default();
    for (index, command) in program.commands.iter().enumerate() {
        collector.command(command, &format!("/program/commands/{index}"));
    }
    for (name, template) in view_templates {
        let at = format!("/view_templates/{name}");
        collector.construct = "artifact";
        collector.add(
            "compiled_view_template",
            "0.16",
            FeatureClass::Extension,
            &at,
        );
        collector.expression(&template.expression, &format!("{at}/expression"));
    }
    for (name, template) in handler_templates {
        let at = format!("/handler_templates/{name}");
        collector.construct = "artifact";
        collector.add(
            "compiled_handler_template",
            "0.18",
            FeatureClass::Deferred,
            &at,
        );
        for (index, binding) in template.recipe.bindings.iter().enumerate() {
            collector.expression(
                &binding.value,
                &format!("{at}/recipe/bindings/{index}/value"),
            );
        }
    }
    let features: Vec<FeatureUse> = collector
        .uses
        .into_iter()
        .map(
            |((class, feature, construct), (since, locations))| FeatureUse {
                feature,
                construct,
                since,
                class,
                locations,
            },
        )
        .collect();
    ContractProfile {
        protocol: program.version.clone(),
        profile: CORE_PROFILE,
        core_only: features.iter().all(|f| f.class == FeatureClass::CoreV1),
        features,
    }
}
