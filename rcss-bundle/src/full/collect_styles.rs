use std::{collections::BTreeMap, io::Write};

// We use this as path dependency because we can't use it as crate
// Since it is depending on rcss which creates circular dependency.
#[path = "../../../rcss-layers/src/lib.rs"]
#[allow(unused)]
pub mod rcss_layers;
use rcss_layers::{ScopeId, Style};

type ModId = Vec<String>;

#[derive(Clone, Debug)]
enum DependencyInfo {
    Computed { extend: ModId },
    Calculated { order: u32, root_scope_id: ScopeId },
}

#[derive(Clone)]
pub struct Collector {
    declared_structs: BTreeMap<ModId, (DependencyInfo, ScopeId, Style)>,
    other_css: Vec<(DependencyInfo, ScopeId, Style)>,
}

#[derive(Debug, Clone, Default)]
pub struct Styles {
    sorted: rcss_layers::LayeredCss,
}
impl Styles {
    fn add_calculated(
        &mut self,
        root_scope_id: ScopeId,
        order: u32,
        scope_id: ScopeId,
        style: Style,
    ) {
        // insert root
        if order == 0 {
            assert_eq!(root_scope_id, scope_id);
            assert!(!self.sorted.root_scope_exist(root_scope_id.clone()));
        }

        self.sorted
            .add_style_from_parts(root_scope_id, order, scope_id, style);
    }

    // We found a style that has dependency on another style.
    // Check if parent style is already resolved, if not, add it to stack
    // If it is resolved, add it to sorted styles, and mark it as resolved.
    fn compute_resolution(
        &mut self,
        stack: Option<(&mut Vec<ModId>, ModId)>,
        extend: ModId,
        scope_id: ScopeId,
        style: Style,
        resolution: &mut BTreeMap<ModId, (DependencyInfo, ScopeId, Style)>,
    ) {
        let (parent_di, _, _) = resolution.get(&extend).expect("Cannot find extended style");
        let (new_order, root_scope_id) = match parent_di {
            DependencyInfo::Calculated {
                order,
                root_scope_id,
            } => {
                let new_order = order + 1;
                (new_order, root_scope_id.clone())
            }
            _ => {
                let Some((stack, file_id)) = stack else {
                    panic!("Some style extend unresolved style")
                };
                // Return back file to stack
                stack.push(file_id);
                // And parent item to stack
                stack.push(extend.clone());
                return;
            }
        };
        if let Some((_, file_id)) = stack {
            // Update of order should be reflected in map
            resolution.insert(
                file_id.clone(),
                (
                    DependencyInfo::Calculated {
                        order: new_order,
                        root_scope_id: root_scope_id.clone(),
                    },
                    scope_id.clone(),
                    style.clone(),
                ),
            );
        }
        // TODO: assert prev is none
        assert!(!self
            .sorted
            .layer_exist_in_root_scope(root_scope_id.clone(), scope_id.clone()));
        self.sorted
            .add_style_from_parts(root_scope_id.clone(), new_order, scope_id, style);
    }
    pub fn from_unsorted(styles: Collector) -> Self {
        let mut unsorted = styles.declared_structs;
        let other_css = styles.other_css;
        let mut styles = Styles::default();

        // Firstly process all computed, then go to calculated
        let (to_compute, calculated): (Vec<_>, Vec<_>) = unsorted
            .clone()
            .into_iter()
            .partition(|(_, (order, _, _))| matches!(order, DependencyInfo::Computed { .. }));

        let mut stack = to_compute
            .into_iter()
            .chain(calculated)
            .map(|(file_id, _)| file_id)
            .collect::<Vec<_>>();

        let mut limit_iters = 1000;
        println!("Start sorting styles :{:?}", unsorted);

        while let Some(file_id) = stack.pop() {
            let (di, scope_id, style) = unsorted.get(&file_id).cloned().unwrap();
            println!("Loading unsorted: {:?}", file_id);
            println!("Dependency Info: {:?}", di);
            println!("Scope id: {:?}", scope_id);
            println!("Style: {:?}", style);

            match di {
                DependencyInfo::Calculated {
                    order,
                    root_scope_id,
                } => {
                    styles.add_calculated(root_scope_id.clone(), order, scope_id, style);
                }
                DependencyInfo::Computed { extend } => {
                    styles.compute_resolution(
                        Some((&mut stack, file_id)),
                        extend,
                        scope_id,
                        style,
                        &mut unsorted,
                    );
                }
            }
            if limit_iters == 0 {
                panic!("Infinite loop detected")
            }
            limit_iters -= 1;
        }
        for (order, scope_id, style) in other_css {
            match order {
                DependencyInfo::Calculated {
                    order,
                    root_scope_id,
                } => {
                    styles.add_calculated(root_scope_id, order, scope_id, style);
                }
                DependencyInfo::Computed { extend } => {
                    styles.compute_resolution(None, extend, scope_id, style, &mut unsorted);
                }
            }
        }
        styles
    }

    pub fn save_with(&self, config: &crate::BundleOption) {
        let mut resulted_style = String::new();
        for (root_scope_id, layers) in self.sorted.styles.iter() {
            resulted_style.push_str(
                &layers
                    .render(false, root_scope_id.clone())
                    .expect("Failed to render style"),
            )
        }

        println!("output: {}", config.output_path);
        let file = std::fs::File::create(&config.output_path).expect("Failed to create file");
        let mut writer = std::io::BufWriter::new(file);
        writer
            .write_all(resulted_style.as_bytes())
            .expect("Failed to write to file");
    }
}
impl Collector {
    pub fn new() -> Self {
        Self {
            declared_structs: BTreeMap::new(),
            other_css: Vec::new(),
        }
    }

    pub fn add_style(
        &mut self,
        file_id: Option<ModId>,
        scope_id: ScopeId,
        style: Style,
        extend: Option<ModId>,
    ) {
        println!("Adding style: {:?}", style);
        println!("File id: {:?}", file_id);
        println!("Extend: {:?}", extend);
        let order = match extend {
            Some(extend) => DependencyInfo::Computed { extend },
            None => DependencyInfo::Calculated {
                order: 0,
                root_scope_id: scope_id.clone(),
            },
        };
        if let Some(file_id) = file_id {
            self.declared_structs
                .insert(file_id, (order, scope_id, style.into()));
        } else {
            self.other_css.push((order, scope_id, style.into()));
        }
    }

    pub fn to_styles(&self) -> Vec<Style> {
        self.declared_structs
            .values()
            .map(|(_, _, style)| style)
            .chain(self.other_css.iter().map(|(_, _, style)| style))
            .cloned()
            .collect()
    }
}
