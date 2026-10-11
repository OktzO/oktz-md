use oktz_plugin_api::{Ctx, Reply, plugin};

/// Template JS yang dibungkus ```` ``` ````. Diawali dan diakhiri newline
/// persis seperti literal di `templateplugin.js`.
pub(crate) const TEMPLATE: &str = r#"
const pluginConfig = {
    name: 'example',
    alias: ['ex'],
    category: 'general',
    description: 'Example plugin',
    usage: '.example',
    example: '.example',
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 3,
    energi: 1,
    isEnabled: true
}
async function handler(m, { sock }) {
    try {
        await m.reply('This is an example plugin!')
    } catch (error) {
        console.error('Example Plugin Error:', error)
        await m.reply('❌ *GAGAL*\n\n> ' + error.message)
    }
}
export { pluginConfig as config, handler }
"#;

#[plugin(
    name = "templateplugin",
    alias = ["tplplugin", "plugin-template"],
    category = "owner",
    description = "Generate plugin template (Owner Only)",
    usage = ".templateplugin",
    example = ".templateplugin",
    cooldown = 0,
    energi = 0,
    is_owner = true,
    is_private = false
)]
pub fn templateplugin(_ctx: Ctx) -> Reply {
    Reply::text(format!("```{TEMPLATE}```"))
}
