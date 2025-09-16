use crate::{Context, FullOutput, RawInput};
use ahash::HashMap;
use epaint::mutex::{Mutex, MutexGuard};
use std::sync::Arc;

/// A plugin to extend egui.
///
/// Add plugins via [`Context::add_plugin`].
///
/// Plugins should not hold a reference to the [`Context`], since this would create a cycle
/// (which would prevent the [`Context`] from being dropped).
#[expect(unused_variables)]
pub trait Plugin: Send + Sync + std::any::Any + 'static {
    /// Plugin name.
    ///
    /// Used when profiling.
    fn debug_name(&self) -> &'static str;

    /// Called once, when the plugin is registered.
    ///
    /// Useful to e.g. register image loaders.
    fn setup(&mut self, ctx: &Context) {}

    /// Called at the start of each pass.
    ///
    /// Can be used to show ui, e.g. a [`crate::Window`] or [`crate::SidePanel`].
    fn on_begin_pass(&mut self, ctx: &Context) {}

    /// Called at the end of each pass.
    ///
    /// Can be used to show ui, e.g. a [`crate::Window`].
    fn on_end_pass(&mut self, ctx: &Context) {}

    /// Called just before the input is processed.
    ///
    /// Useful to inspect or modify the input.
    /// Since this is called outside a pass, don't show ui here.
    fn input_hook(&mut self, input: &mut RawInput) {}

    /// Called just before the output is passed to the backend.
    ///
    /// Useful to inspect or modify the output.
    /// Since this is called outside a pass, don't show ui here.
    fn output_hook(&mut self, output: &mut FullOutput) {}
}

pub(crate) struct PluginHandle {
    plugin: Box<dyn Plugin>,
}

pub struct TypedPluginHandle<P: Plugin> {
    handle: Arc<Mutex<PluginHandle>>,
    _type: std::marker::PhantomData<P>,
}

impl<P: Plugin> TypedPluginHandle<P> {
    pub(crate) fn new(handle: Arc<Mutex<PluginHandle>>) -> Self {
        Self {
            handle,
            _type: std::marker::PhantomData,
        }
    }

    pub fn lock(&self) -> TypedPluginGuard<'_, P> {
        TypedPluginGuard {
            guard: self.handle.lock(),
            _type: std::marker::PhantomData,
        }
    }
}

pub struct TypedPluginGuard<'a, P: Plugin> {
    guard: MutexGuard<'a, PluginHandle>,
    _type: std::marker::PhantomData<P>,
}

impl<P: Plugin> TypedPluginGuard<'_, P> {}

impl<P: Plugin> std::ops::Deref for TypedPluginGuard<'_, P> {
    type Target = P;

    fn deref(&self) -> &Self::Target {
        self.guard.typed_plugin()
    }
}

impl<P: Plugin> std::ops::DerefMut for TypedPluginGuard<'_, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.typed_plugin_mut()
    }
}

impl PluginHandle {
    pub fn new<P: Plugin>(plugin: P) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            plugin: Box::new(plugin),
        }))
    }

    fn plugin_type_id(&self) -> std::any::TypeId {
        (*self.plugin).type_id()
    }

    pub fn dyn_plugin_mut(&mut self) -> &mut dyn Plugin {
        &mut *self.plugin
    }

    fn typed_plugin<P: Plugin + 'static>(&self) -> &P {
        (&*self.plugin as &dyn std::any::Any)
            .downcast_ref::<P>()
            .expect("PluginHandle: plugin is not of the expected type")
    }

    pub fn typed_plugin_mut<P: Plugin + 'static>(&mut self) -> &mut P {
        (&mut *self.plugin as &mut dyn std::any::Any)
            .downcast_mut::<P>()
            .expect("PluginHandle: plugin is not of the expected type")
    }
}

/// User-registered plugins.
#[derive(Clone, Default)]
pub(crate) struct Plugins {
    plugins: HashMap<std::any::TypeId, Arc<Mutex<PluginHandle>>>,
    plugins_ordered: PluginsOrdered,
}

#[derive(Clone, Default)]
pub(crate) struct PluginsOrdered(Vec<Arc<Mutex<PluginHandle>>>);

impl PluginsOrdered {
    fn for_each_dyn<F>(&self, mut f: F)
    where
        F: FnMut(&mut dyn Plugin),
    {
        for plugin in &self.0 {
            let mut plugin = plugin.lock();
            profiling::scope!("plugin", plugin.dyn_plugin_mut().debug_name());
            f(plugin.dyn_plugin_mut());
        }
    }

    pub fn on_begin_pass(&self, ctx: &Context) {
        profiling::scope!("plugins", "on_begin_pass");
        self.for_each_dyn(|p| {
            p.on_begin_pass(ctx);
        });
    }

    pub fn on_end_pass(&self, ctx: &Context) {
        profiling::scope!("plugins", "on_end_pass");
        self.for_each_dyn(|p| {
            p.on_end_pass(ctx);
        });
    }

    pub fn on_input(&self, input: &mut RawInput) {
        profiling::scope!("plugins", "on_input");
        self.for_each_dyn(|plugin| {
            plugin.input_hook(input);
        });
    }

    pub fn on_output(&self, output: &mut FullOutput) {
        profiling::scope!("plugins", "on_output");
        self.for_each_dyn(|plugin| {
            plugin.output_hook(output);
        });
    }
}

impl Plugins {
    pub fn ordered_plugins(&self) -> PluginsOrdered {
        self.plugins_ordered.clone()
    }

    /// Remember to call [`Plugin::setup`] on the plugin after adding it.
    ///
    /// Will not add the plugin if a plugin of the same type already exists.
    /// Returns `false` if the plugin was not added, `true` if it was added.
    pub fn add(&mut self, handle: Arc<Mutex<PluginHandle>>) -> bool {
        profiling::scope!("plugins", "add");

        let type_id = handle.lock().plugin_type_id();

        if self.plugins.contains_key(&type_id) {
            return false;
        }

        self.plugins.insert(type_id, handle.clone());
        self.plugins_ordered.0.push(handle);

        true
    }

    pub fn get(&self, type_id: std::any::TypeId) -> Option<Arc<Mutex<PluginHandle>>> {
        self.plugins.get(&type_id).cloned()
    }
}

/// Generic event callback.
pub type ContextCallback = Arc<dyn Fn(&Context) + Send + Sync>;

#[derive(Default)]
pub(crate) struct CallbackPlugin {
    pub on_begin_plugins: Vec<(&'static str, ContextCallback)>,
    pub on_end_plugins: Vec<(&'static str, ContextCallback)>,
}

impl Plugin for CallbackPlugin {
    fn debug_name(&self) -> &'static str {
        "CallbackPlugins"
    }

    fn on_begin_pass(&mut self, ctx: &Context) {
        profiling::function_scope!();

        for (_debug_name, cb) in &self.on_begin_plugins {
            profiling::scope!(*_debug_name);
            (cb)(ctx);
        }
    }

    fn on_end_pass(&mut self, ctx: &Context) {
        profiling::function_scope!();

        for (_debug_name, cb) in &self.on_end_plugins {
            profiling::scope!(*_debug_name);
            (cb)(ctx);
        }
    }
}
