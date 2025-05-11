use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, Weak};
use std::sync::atomic::{AtomicBool, Ordering};
use std::marker::PhantomData;
use crate::event::Event;

/// The heart of the event system. Significant game events are sent/received here.
///
/// Use `send` to send events and `EventReceiver::receive` to receive events.
///
/// **Do not use this for every communication between modules**. Only use it for events that might be relevant for a wide variety of modules or
/// significantly affect the game state, i.e. buildings being created, units dying, new multiplayer data available, etc.
pub struct EventBus {
    listeners: Arc<Mutex<HashMap<TypeId, Vec<EventListenerWeakReference>>>>,
}

impl EventBus {
    /// Creates a new EventBus
    pub fn new() -> Self {
        Self {
            listeners: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Sends an event to all registered listeners.
    ///
    /// Only use this from the render thread. For example, in coroutines launched by crash handling
    /// always wrap the call in post_crash_handling_runnable.
    ///
    /// We could use a generic method like `send_on_render_thread` or make the whole event system asynchronous in general,
    /// but doing it like this makes debugging slightly easier.
    pub fn send<T: Event + 'static>(&self, event: T) {
        let event_type_id = TypeId::of::<T>();
        let event_listeners = self.get_listeners(event_type_id);

        for listener in event_listeners {
            if let Some(filter) = &listener.filter {
                if !filter(&event) {
                    continue;
                }
            }

            if let Some(handler) = listener.event_handler.upgrade() {
                handler(&event);
            }
        }
    }

    /// Gets all listeners for a specific event type
    fn get_listeners<T: Event + 'static>(&self, event_type_id: TypeId) -> Vec<EventListener<T>> {
        let classes_to_listen_to = self.get_classes_to_listen_to(event_type_id);

        // Set because we don't want to notify the same listener multiple times
        let mut result = HashSet::new();

        for class_to_listen_to in classes_to_listen_to {
            let active_listeners = self.update_active_listeners(class_to_listen_to);
            result.extend(active_listeners);
        }

        result.into_iter().collect()
    }

    /// To be able to listen to an event class and get notified even when child classes are sent as an event
    fn get_classes_to_listen_to(&self, event_type_id: TypeId) -> Vec<TypeId> {
        // In Rust, we can't easily get supertypes at runtime like in Kotlin
        // For simplicity, we'll just return the event type itself
        // In a more complex implementation, we could use a registry of event hierarchies
        vec![event_type_id]
    }

    /// Removes all listeners whose WeakReference got collected and returns the ones that are still active
    fn update_active_listeners(&self, event_type_id: TypeId) -> Vec<EventListener<dyn Event>> {
        let mut result = Vec::new();

        if let Ok(mut listeners) = self.listeners.lock() {
            if let Some(listeners_weak) = listeners.get(&event_type_id) {
                let mut active_listeners = Vec::new();

                for listener in listeners_weak {
                    if let Some(handler) = listener.event_handler.upgrade() {
                        active_listeners.push(EventListener {
                            event_handler: handler,
                            filter: listener.filter.upgrade(),
                        });
                    }
                }

                // Update the listeners list to remove inactive ones
                if active_listeners.len() != listeners_weak.len() {
                    listeners.insert(event_type_id, active_listeners.iter()
                        .map(|l| EventListenerWeakReference {
                            event_handler: Arc::downgrade(&l.event_handler),
                            filter: l.filter.as_ref().map(|f| Arc::downgrade(f)),
                        })
                        .collect());
                }

                result = active_listeners;
            }
        }

        result
    }

    /// Registers a new event listener
    fn receive<T: Event + 'static>(&self, filter: Option<Arc<dyn Fn(&T) -> bool + Send + Sync>>, event_handler: Arc<dyn Fn(&T) + Send + Sync>) {
        let event_type_id = TypeId::of::<T>();

        if let Ok(mut listeners) = self.listeners.lock() {
            let entry = listeners.entry(event_type_id).or_insert_with(Vec::new);

            entry.push(EventListenerWeakReference {
                event_handler: Arc::downgrade(&event_handler),
                filter: filter.map(|f| Arc::downgrade(&f)),
            });
        }
    }

    /// Cleans up event handlers
    fn clean_up(&self, event_handlers: &HashMap<TypeId, Vec<Arc<dyn Any + Send + Sync>>>) {
        if let Ok(mut listeners) = self.listeners.lock() {
            for (type_id, to_remove) in event_handlers {
                if let Some(registered_listeners) = listeners.get_mut(type_id) {
                    registered_listeners.retain(|listener| {
                        if let Some(handler) = listener.event_handler.upgrade() {
                            !to_remove.iter().any(|remove| {
                                remove.as_any().downcast_ref::<Arc<dyn Fn(&dyn Event) + Send + Sync>>()
                                    .map_or(false, |f| Arc::ptr_eq(f, &handler))
                            })
                        } else {
                            false
                        }
                    });
                }
            }
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Used to receive events by the EventBus.
///
/// Usage:
///
/// ```
/// struct SomeStruct {
///     events: EventReceiver,
/// }
///
/// impl SomeStruct {
///     fn new() -> Self {
///         let mut this = Self {
///             events: EventReceiver::new(),
///         };
///
///         this.events.receive::<SomeEvent>(None, Arc::new(|event| {
///             // do something when the event is received.
///         }));
///
///         this
///     }
///
///     // Optional
///     fn cleanup(&mut self) {
///         self.events.stop_receiving();
///     }
/// }
/// ```
///
/// The `stop_receiving` call is optional. Event listeners will be automatically garbage collected. However, garbage collection is non-deterministic, so it's
/// possible that the events keep being received for quite a while even after a struct is unused. `stop_receiving` immediately cleans up all listeners.
///
/// To have event listeners automatically garbage collected, we need to use `Weak` references in the event bus. For that to work, though, the struct
/// that wants to receive events needs to hold references to its own event listeners. `EventReceiver` allows to do that while also providing the
/// interface to start receiving events.
pub struct EventReceiver {
    event_handlers: HashMap<TypeId, Vec<Arc<dyn Any + Send + Sync>>>,
    filters: Vec<Arc<dyn Any + Send + Sync>>,
    event_bus: Arc<EventBus>,
}

impl EventReceiver {
    /// Creates a new EventReceiver
    pub fn new() -> Self {
        Self {
            event_handlers: HashMap::new(),
            filters: Vec::new(),
            event_bus: Arc::new(EventBus::new()),
        }
    }

    /// Listen to the event with the given type and all events that implement it. Use `stop_receiving` to stop listening to all events.
    ///
    /// The listeners will always be called on the main render thread.
    pub fn receive<T: Event + 'static>(&mut self, filter: Option<Box<dyn Fn(&T) -> bool + Send + Sync>>, event_handler: Arc<dyn Fn(&T) + Send + Sync>) {
        let type_id = TypeId::of::<T>();

        if let Some(filter) = filter {
            self.filters.push(Arc::new(filter) as Arc<dyn Any + Send + Sync>);
        }

        let entry = self.event_handlers.entry(type_id).or_insert_with(Vec::new);
        entry.push(Arc::new(event_handler.clone()) as Arc<dyn Any + Send + Sync>);

        self.event_bus.receive(
            filter.map(|f| Arc::new(f) as Arc<dyn Fn(&T) -> bool + Send + Sync>),
            event_handler,
        );
    }

    /// Stops receiving all events, cleaning up all event listeners.
    pub fn stop_receiving(&mut self) {
        self.event_bus.clean_up(&self.event_handlers);
        self.event_handlers.clear();
        self.filters.clear();
    }
}

impl Default for EventReceiver {
    fn default() -> Self {
        Self::new()
    }
}

/// Exists so that eventHandlers and filters do not get garbage-collected *while* we are passing them around in here,
/// otherwise we would only need `EventListenerWeakReference`
struct EventListener<T: ?Sized> {
    event_handler: Arc<dyn Fn(&T) + Send + Sync>,
    filter: Option<Arc<dyn Fn(&T) -> bool + Send + Sync>>,
}

/// Weak reference to an event listener
struct EventListenerWeakReference {
    event_handler: Weak<dyn Fn(&dyn Event) + Send + Sync>,
    filter: Option<Weak<dyn Fn(&dyn Event) -> bool + Send + Sync>>,
}

/// Global event bus instance
lazy_static::lazy_static! {
    pub static ref EVENT_BUS: Arc<EventBus> = Arc::new(EventBus::new());
}

/// Sends an event to all registered listeners.
pub fn send<T: Event + 'static>(event: T) {
    EVENT_BUS.send(event);
}