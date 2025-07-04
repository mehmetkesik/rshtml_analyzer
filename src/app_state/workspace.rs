use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;
use tracing::{debug, info};

pub struct Workspace {
    pub root: PathBuf,
    pub members: Vec<Member>,
}

pub struct Member {
    pub path: PathBuf,
    pub views_path: PathBuf,
    pub views_layout: String,
}

impl Default for Workspace {
    fn default() -> Self {
        Workspace {
            root: PathBuf::new(),
            members: Vec::new(),
        }
    }
}

impl Workspace {
    pub fn load(&mut self, root: &Path) -> Result<(), String> {
        let mut new_workspace = Self::default();
        new_workspace.root = root.to_path_buf();

        let cargo_toml = fs::read_to_string(&root.join("Cargo.toml")).map_err(|e| e.to_string())?;
        let cargo_toml: Value = toml::from_str(&cargo_toml).map_err(|e| e.to_string())?;

        info!("ROOT CARGO TOML: {}", cargo_toml);

        let member_paths = cargo_toml
            .get("workspace")
            .and_then(|workspace| {
                workspace
                    .get("members")
                    .and_then(|members| members.as_array())
            })
            .and_then(|members| {
                Some(
                    members
                        .iter()
                        .map(|member| root.join(member.to_string().trim_matches('"')))
                        .collect::<Vec<_>>(),
                )
            });

        if let Some(member_paths) = member_paths {
            for member_path in member_paths {
                debug!("MEMBER PATH: {}", member_path.to_str().unwrap());
                let cargo_toml = fs::read_to_string(&member_path.join("Cargo.toml"))
                    .map_err(|e| e.to_string())?;
                let cargo_toml: Value = toml::from_str(&cargo_toml).map_err(|e| e.to_string())?;
                let views = self.load_manifest(&cargo_toml)?;
                let member = Member {
                    path: member_path,
                    views_path: root.join(views.0),
                    views_layout: views.1.to_string(),
                };

                new_workspace.members.push(member);
            }
        } else {
            let views = self.load_manifest(&cargo_toml)?;
            let member = Member {
                path: root.to_path_buf(),
                views_path: root.join(views.0),
                views_layout: views.1.to_string(),
            };
            debug!("MEMBER PATH: {}", member.path.to_str().unwrap());

            new_workspace.members.push(member);
        }

        self.root = new_workspace.root;
        self.members = new_workspace.members;

        Ok(())
    }

    fn load_manifest<'a>(&self, cargo_toml: &'a Value) -> Result<(&'a str, &'a str), String> {
        let default_path = "views";
        let default_layout = "layout.rs.html";
        match cargo_toml
            .get("package.metadata.rshtml")
            .and_then(|x| x.get("views"))
        {
            Some(x) => {
                let path = x
                    .get("path")
                    .and_then(|x| x.as_str())
                    .unwrap_or(default_path);
                let layout = x
                    .get("layout")
                    .and_then(|x| x.as_str())
                    .unwrap_or(default_layout);
                Ok((path, layout))
            }
            None => Ok((default_path, default_layout)),
        }
    }

    pub fn get_member_by_view(&self, view_path: &Path) -> Option<&Member> {
        let mut manifest_path: PathBuf = view_path.to_path_buf();

        for current_dir in view_path.ancestors() {
            if current_dir.join("Cargo.toml").exists() {
                manifest_path = current_dir.to_path_buf();
            }

            if current_dir == self.root {
                return None;
            }
        }

        for member in &self.members {
            if member.path == manifest_path {
                return Some(member);
            }
        }

        None
    }
}
