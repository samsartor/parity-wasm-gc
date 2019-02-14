use crate::rust::{fmt, vec::Vec};
use crate::io;
use super::{
	Deserialize, Serialize, Error, VarUint7, VarInt7, VarUint1, CountedList,
	CountedListWriter, VarUint32,
};

/// Type definition in types section. Currently can be only of the function type.
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum Type {
	/// Function type.
	Function(FunctionType),
}

impl Deserialize for Type {
	type Error = Error;

	fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
		Ok(Type::Function(FunctionType::deserialize(reader)?))
	}
}

impl Serialize for Type {
	type Error = Error;

	fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
		match self {
			Type::Function(fn_type) => fn_type.serialize(writer)
		}
	}
}

/// Value type.
#[derive(Clone, Copy, Debug, PartialEq, Hash, Eq)]
pub enum ValueType {
	/// Number type
	Num(NumType),
	/// Reference type
	Ref(RefType),
	/// 128-bit SIMD register
	V128,
}

impl From<RefType> for ValueType {
	fn from(r: RefType) -> ValueType {
		ValueType::Ref(r)
	}
}

impl From<NumType> for ValueType {
	fn from(n: NumType) -> ValueType {
		ValueType::Num(n)
	}
}

impl ValueType {
	fn from_bits(x: i8) -> Option<ValueType> {
		match x {
			-0x05 => Some(ValueType::V128),
			_ => None,
		}
		.or(NumType::from_bits(x).map(Into::into))
		.or(RefType::from_bits(x).map(Into::into))
	}

	fn to_bits(self) -> i8 {
		match self {
			ValueType::V128 => -0x05,
			ValueType::Num(n) => n.to_bits(),
			ValueType::Ref(r) => r.to_bits(),
		}
	}
}

/// Reference type.
#[derive(Clone, Copy, Debug, PartialEq, Hash, Eq)]
pub enum RefType {
	/// Infinite union of all references
	AnyRef,
	/// Infinite union of all references to functions
	AnyFunc,
}

impl RefType {
	fn from_bits(x: i8) -> Option<RefType> {
		match x {
			-0x10 => Some(RefType::AnyFunc),
			-0x11 => Some(RefType::AnyRef),
			_ => None,
		}
	}

	fn to_bits(self) -> i8 {
		match self {
			RefType::AnyFunc => -0x10,
			RefType::AnyRef => -0x11,
		}
	}
}

impl Deserialize for RefType {
	type Error = Error;

	fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
		let val = VarInt7::deserialize(reader)?;
		RefType::from_bits(val.into())
			.ok_or(Error::UnknownValueType(val.into()))
	}
}

impl Serialize for RefType {
	type Error = Error;

	fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
		let val: VarInt7 = self.to_bits().into();
		val.serialize(writer)?;
		Ok(())
	}
}

/// Number type.
#[derive(Clone, Copy, Debug, PartialEq, Hash, Eq)]
pub enum NumType {
	/// 32-bit signed integer
	I32,
	/// 64-bit signed integer
	I64,
	/// 32-bit float
	F32,
	/// 64-bit float
	F64,
}

impl NumType {
	fn from_bits(x: i8) -> Option<NumType> {
		match x {
			-0x01 => Some(NumType::I32),
			-0x02 => Some(NumType::I64),
			-0x03 => Some(NumType::F32),
			-0x04 => Some(NumType::F64),
			_ => None,
		}
	}

	fn to_bits(self) -> i8 {
		match self {
			NumType::I32 => -0x01,
			NumType::I64 => -0x02,
			NumType::F32 => -0x03,
			NumType::F64 => -0x04,
		}
	}
}

impl Deserialize for ValueType {
	type Error = Error;

	fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
		let val = VarInt7::deserialize(reader)?;
		ValueType::from_bits(val.into())
			.ok_or(Error::UnknownValueType(val.into()))
	}
}

impl Serialize for ValueType {
	type Error = Error;

	fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
		let val: VarInt7 = self.to_bits().into();
		val.serialize(writer)?;
		Ok(())
	}
}

impl fmt::Display for ValueType {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			ValueType::Num(NumType::I32) => write!(f, "i32"),
			ValueType::Num(NumType::I64) => write!(f, "i64"),
			ValueType::Num(NumType::F32) => write!(f, "f32"),
			ValueType::Num(NumType::F64) => write!(f, "f64"),
			ValueType::Ref(RefType::AnyRef) => write!(f, "anyref"),
			ValueType::Ref(RefType::AnyFunc) => write!(f, "anyfunc"),
			ValueType::V128 => write!(f, "v128"),
		}
	}
}

/// Block type which is basically `ValueType` + NoResult (to define blocks that have no return type)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BlockType {
	/// Value-type specified block type
	Value(ValueType),
	/// No specified block type
	NoResult,
}

impl Deserialize for BlockType {
	type Error = Error;

	fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
		let val = VarInt7::deserialize(reader)?;
		let bits = val.into();
		match bits {
			-0x40 => Some(BlockType::NoResult),
			_ => None,
		}
		.or(ValueType::from_bits(bits).map(BlockType::Value))
		.ok_or(Error::UnknownValueType(val.into()))
	}
}

impl Serialize for BlockType {
	type Error = Error;

	fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
		let val: VarInt7 = match self {
			BlockType::NoResult => -0x40i8,
			BlockType::Value(v) => v.to_bits(),
		}.into();
		val.serialize(writer)?;
		Ok(())
	}
}

/// Function signature type.
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct FunctionType {
	form: u8,
	params: Vec<ValueType>,
	return_type: Option<ValueType>,
}

impl Default for FunctionType {
	fn default() -> Self {
		FunctionType {
			form: 0x60,
			params: Vec::new(),
			return_type: None,
		}
	}
}

impl FunctionType {
	/// New function type given the signature in-params(`params`) and return type (`return_type`)
	pub fn new(params: Vec<ValueType>, return_type: Option<ValueType>) -> Self {
		FunctionType {
			params: params,
			return_type: return_type,
			..Default::default()
		}
	}
	/// Function form (currently only valid value is `0x60`)
	pub fn form(&self) -> u8 { self.form }
	/// Parameters in the function signature.
	pub fn params(&self) -> &[ValueType] { &self.params }
	/// Mutable parameters in the function signature.
	pub fn params_mut(&mut self) -> &mut Vec<ValueType> { &mut self.params }
	/// Return type in the function signature, if any.
	pub fn return_type(&self) -> Option<ValueType> { self.return_type }
	/// Mutable type in the function signature, if any.
	pub fn return_type_mut(&mut self) -> &mut Option<ValueType> { &mut self.return_type }
}

impl Deserialize for FunctionType {
	type Error = Error;

	fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
		let form: u8 = VarUint7::deserialize(reader)?.into();

		if form != 0x60 {
			return Err(Error::UnknownFunctionForm(form));
		}

		let params: Vec<ValueType> = CountedList::deserialize(reader)?.into_inner();

		let return_types: u32 = VarUint32::deserialize(reader)?.into();

		let return_type = if return_types == 1 {
			Some(ValueType::deserialize(reader)?)
		} else if return_types == 0 {
			None
		} else {
			return Err(Error::Other("Return types length should be 0 or 1"));
		};

		Ok(FunctionType {
			form: form,
			params: params,
			return_type: return_type,
		})
	}
}

impl Serialize for FunctionType {
	type Error = Error;

	fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
		VarUint7::from(self.form).serialize(writer)?;

		let data = self.params;
		let counted_list = CountedListWriter::<ValueType, _>(
			data.len(),
			data.into_iter().map(Into::into),
		);
		counted_list.serialize(writer)?;

		if let Some(return_type) = self.return_type {
			VarUint1::from(true).serialize(writer)?;
			return_type.serialize(writer)?;
		} else {
			VarUint1::from(false).serialize(writer)?;
		}

		Ok(())
	}
}

