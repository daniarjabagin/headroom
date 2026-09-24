use crate::model::Model;
use crate::state::payload::UpdateView;

pub fn update_view(model: &Model) -> Option<UpdateView> {
    if !model.settings.updates.check {
        return None;
    }
    let update = model.update.as_ref()?;
    Some(UpdateView {
        version: update.release.version.to_string(),
        url: update.release.url.clone(),
        published_at: update.release.published_at,
        install: update.install.kind(),
        command: update.install.command(&update.release),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update::{AvailableUpdate, Install, InstallKind, Packager, Release};

    fn with_update(install: Install) -> Model {
        Model {
            update: Some(AvailableUpdate {
                release: Release {
                    version: "0.5.0".parse().unwrap(),
                    url: "https://github.com/daniarjabagin/headroom/releases/tag/v0.5.0".into(),
                    published_at: "2026-10-01T09:20:02Z".parse().unwrap(),
                },
                install,
            }),
            ..Model::default()
        }
    }

    #[test]
    fn a_known_update_serializes_to_the_contract_shape() {
        let view = update_view(&with_update(Install::Script)).unwrap();
        assert_eq!(
            serde_json::to_value(&view).unwrap(),
            serde_json::json!({
                "version": "0.5.0",
                "url": "https://github.com/daniarjabagin/headroom/releases/tag/v0.5.0",
                "published_at": "2026-10-01T09:20:02Z",
                "install": "self",
                "command": "headroom update"
            })
        );
        let packaged = update_view(&with_update(Install::Package(Packager::Deb))).unwrap();
        assert_eq!(packaged.install, InstallKind::Package);
    }

    #[test]
    fn no_update_is_shown_without_one_or_with_checks_off() {
        assert_eq!(update_view(&Model::default()), None);
        let mut model = with_update(Install::Unknown);
        model.settings.updates.check = false;
        assert_eq!(update_view(&model), None);
    }
}
