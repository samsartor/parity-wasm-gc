use crate::rust::vec::Vec;
use super::invoke::{Invoke, Identity};
use crate::elements;

pub struct ValueTypeBuilder<F=Identity> {
	callback: F,
}

impl<F> ValueTypeBuilder<F> where F: Invoke<elements::ValueType> {
	pub fn with_callback(callback: F) -> Self {
		ValueTypeBuilder { callback: callback }
	}

	pub fn i32(self) -> F::Result {
		self.callback.invoke(elements::NumType::I32.into())
	}

	pub fn i64(self) -> F::Result {
		self.callback.invoke(elements::NumType::I64.into())
	}

	pub fn f32(self) -> F::Result {
		self.callback.invoke(elements::NumType::F32.into())
	}

	pub fn f64(self) -> F::Result {
		self.callback.invoke(elements::NumType::F64.into())
	}
}

pub struct OptionalValueTypeBuilder<F=Identity> {
	callback: F,
}

impl<F> OptionalValueTypeBuilder<F> where F: Invoke<Option<elements::ValueType>> {
	pub fn with_callback(callback: F) -> Self {
		OptionalValueTypeBuilder { callback: callback }
	}

	pub fn i32(self) -> F::Result {
		self.callback.invoke(Some(elements::NumType::I32.into()))
	}

	pub fn i64(self) -> F::Result {
		self.callback.invoke(Some(elements::NumType::I64.into()))
	}

	pub fn f32(self) -> F::Result {
		self.callback.invoke(Some(elements::NumType::F32.into()))
	}

	pub fn f64(self) -> F::Result {
		self.callback.invoke(Some(elements::NumType::F64.into()))
	}
}

pub struct ValueTypesBuilder<F=Identity> {
	callback: F,
	value_types: Vec<elements::ValueType>,
}

impl<F> ValueTypesBuilder<F> where F: Invoke<Vec<elements::ValueType>> {
	pub fn with_callback(callback: F) -> Self {
		ValueTypesBuilder {
			callback: callback,
			value_types: Vec::new(),
		}
	}

	pub fn i32(mut self) -> Self {
		self.value_types.push(elements::NumType::I32.into());
		self
	}

	pub fn i64(mut self) -> Self {
		self.value_types.push(elements::NumType::I64.into());
		self
	}

	pub fn f32(mut self) -> Self {
		self.value_types.push(elements::NumType::F32.into());
		self
	}

	pub fn f64(mut self) -> Self {
		self.value_types.push(elements::NumType::F64.into());
		self
	}

	pub fn build(self) -> F::Result {
		self.callback.invoke(self.value_types)
	}
}
