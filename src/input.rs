use std::{cell::RefCell, collections::BTreeMap, rc::Rc, sync::OnceLock};

use bevy_input::{
	ButtonState,
	keyboard::{Key, KeyboardInput, NativeKey, NativeKeyCode},
	mouse::{MouseButtonInput, MouseMotion},
};
use bevy_reflect::{DynamicEnum, DynamicVariant, PartialReflect, Typed};
use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::{KeyboardEvent, MouseEvent};

use crate::{DomElements, prelude::*};

enum AnyInput {
	Key(KeyboardInput),
	Button(MouseButtonInput),
	Motion(MouseMotion),
}

#[derive(Default)]
struct PendingInputs(Vec<AnyInput>);

type PendingInputsRef = Rc<RefCell<PendingInputs>>;

fn get_keycode_map() -> &'static BTreeMap<String, KeyCode> {
	static CACHE: OnceLock<BTreeMap<String, KeyCode>> = OnceLock::new();
	CACHE.get_or_init(|| {
		let typeinfo = KeyCode::type_info()
			.as_enum()
			.unwrap_or_else(|err| unreachable!("KeyCode should be an enum ({err:?})"));
		typeinfo
			.variant_names()
			.into_iter()
			.map(|s| s.to_string())
			.filter_map(|name| {
				if name == "Unidentified" {
					return None;
				}

				let mut keycode = KeyCode::KeyA;
				let variant = DynamicEnum::new(name.clone(), DynamicVariant::Unit);
				keycode
					.try_apply(variant.as_partial_reflect())
					.expect("could not construct KeyCode variant for keycode cache");
				Some((name, keycode))
			})
			.collect()
	})
}

app_setup_fn!(setup);
fn setup(app: &mut App) -> JsResult {
	app.add_systems(PreUpdate, process_inputs);

	let pending_input = Rc::new(RefCell::new(PendingInputs::default()));
	app.insert_non_send_resource(pending_input.clone());
	let dom_elements: &DomElements = app.world().non_send_resource();

	macro_rules! add_event_listener {
		($handler:ident: $ty:ty; $($event:expr),*) => {
			let handler = Closure::<dyn Fn($ty) -> JsResult>::new($handler);
			$(dom_elements.window.add_event_listener_with_callback($event, handler.as_ref().unchecked_ref())?;)*
			handler.forget();
		};
	}

	let key_handler = {
		let pending_input = pending_input.clone();
		move |event: KeyboardEvent| {
			event.prevent_default();

			let (key_code, logical_key) = {
				let name = event.code();
				let key_code = get_keycode_map()
					.get(&name)
					.copied()
					.unwrap_or(KeyCode::Unidentified(NativeKeyCode::Unidentified));
				let logical_key = Key::Unidentified(NativeKey::Web(name.into()));
				(key_code, logical_key)
			};
			let repeat = event.repeat();
			let state = match event.type_().as_str() {
				"keydown" => ButtonState::Pressed,
				"keyup" => ButtonState::Released,
				_ => unreachable!(),
			};
			pending_input
				.borrow_mut()
				.0
				.push(AnyInput::Key(KeyboardInput {
					key_code,
					logical_key,
					repeat,
					state,
					text: None,
					window: Entity::PLACEHOLDER,
				}));
			Ok(())
		}
	};
	add_event_listener!(key_handler: KeyboardEvent; "keydown", "keyup");

	let button_handler = {
		let pending_input = pending_input.clone();
		move |event: MouseEvent| {
			event.prevent_default();

			let button = match event.button() {
				0 => MouseButton::Left,
				1 => MouseButton::Middle,
				2 => MouseButton::Right,
				x if x > 0 => MouseButton::Other(x as u16),
				_ => {
					log::error!("unknown mouse button numbered {}", event.button());
					return Ok(());
				},
			};
			let state = match event.type_().as_str() {
				"mousedown" => ButtonState::Pressed,
				"mouseup" => ButtonState::Released,
				_ => unreachable!(),
			};
			pending_input
				.borrow_mut()
				.0
				.push(AnyInput::Button(MouseButtonInput {
					button,
					state,
					window: Entity::PLACEHOLDER,
				}));

			Ok(())
		}
	};
	add_event_listener!(button_handler: MouseEvent; "mousedown", "mouseup");

	// prevents menu popup on right click
	// TODO: might also be solved by cursor capture?
	let context_menu_handler = |event: web_sys::Event| {
		event.prevent_default();
		Ok(())
	};
	add_event_listener!(context_menu_handler: web_sys::Event; "contextmenu");

	let button_handler = {
		let pending_input = pending_input.clone();
		move |event: MouseEvent| {
			event.prevent_default();

			let delta = IVec2::new(event.movement_x(), event.movement_y()).as_vec2();
			pending_input
				.borrow_mut()
				.0
				.push(AnyInput::Motion(MouseMotion { delta }));
			Ok(())
		}
	};
	add_event_listener!(button_handler: MouseEvent; "mousemove");

	Ok(())
}

fn process_inputs(
	pending_inputs: NonSendMut<PendingInputsRef>,
	mut keyboard: EventWriter<KeyboardInput>,
	mut mouse: EventWriter<MouseButtonInput>,
	mut motion: EventWriter<MouseMotion>,
) {
	let mut pending_inputs = pending_inputs.borrow_mut();
	for input in pending_inputs.0.drain(..) {
		match input {
			AnyInput::Key(input) => {
				keyboard.write(input);
			},
			AnyInput::Button(input) => {
				mouse.write(input);
			},
			AnyInput::Motion(input) => {
				motion.write(input);
			},
		}
	}
}
