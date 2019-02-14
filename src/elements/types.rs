use crate::rust::{fmt, vec::Vec};
use crate::io;
use super::{
	Deserialize, Serialize, Error, VarInt7, VarUint1, CountedList,
	CountedListWriter, VarUint32,
};

const I32TYPE: i8 = -0x01;
const I64TYPE: i8 = -0x02;
const F32TYPE: i8 = -0x03;
const F64TYPE: i8 = -0x04;
const V128TYPE: i8 = -0x05;
const ANYFUNCTYPE: i8 = -0x10;
const ANYREFTYPE: i8 = -0x11;
const REFTYPE: i8 = -0x12;
const PACKEDI8TYPE: i8 = -0x18;
const PACKEDI16TYPE: i8 = -0x19;
const FUNCTIONTYPE: i8 = -0x20;
const STRUCTTYPE: i8 = -0x21;
const ARRAYTYPE: i8 = -0x22;
const NORESULTTYPE: i8 = -0x40;

/// Type definition in types section. Currently can be only of the function type.
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum Type {
	/// Function type.
	Function(FunctionType),
	/// Structure type.
	Struct(StructType),
	/// ArrayType.
	Array(ArrayType),
}

impl From<FunctionType> for Type {
	fn from(x: FunctionType) -> Type { Type::Function(x) }
}

impl From<StructType> for Type {
	fn from(x: StructType) -> Type { Type::Struct(x) }
}

impl From<ArrayType> for Type {
	fn from(x: ArrayType) -> Type { Type::Array(x) }
}

impl Deserialize for Type {
	type Error = Error;

	fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
		let val = VarInt7::deserialize(reader)?.into();
		match val {
			FUNCTIONTYPE => FunctionType::deserialize(reader).map(Into::into),
			STRUCTTYPE => StructType::deserialize(reader).map(Into::into),
			ARRAYTYPE => ArrayType::deserialize(reader).map(Into::into),
			_ => Err(Error::UnknownValueType(val.into())),
		}
	}
}

impl Serialize for Type {
	type Error = Error;

	fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
		match self {
			Type::Function(fn_type) => {
				VarInt7::from(FUNCTIONTYPE).serialize(writer)?;
				fn_type.serialize(writer)
			},
			Type::Struct(stuct_type) => {
				VarInt7::from(STRUCTTYPE).serialize(writer)?;
				stuct_type.serialize(writer)
			},
			Type::Array(arr_type) => {
				VarInt7::from(ARRAYTYPE).serialize(writer)?;
				arr_type.serialize(writer)
			},
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
			V128TYPE => Some(ValueType::V128),
			_ => None,
		}
		.or(NumType::from_bits(x).map(Into::into))
		.or(RefType::from_bits(x).map(Into::into))
	}

	fn to_bits(self) -> i8 {
		match self {
			ValueType::V128 => V128TYPE,
			ValueType::Num(n) => n.to_bits(),
			ValueType::Ref(r) => r.to_bits(),
		}
	}

	fn read_rest(&mut self, reader: &mut impl io::Read) -> Result<(), Error> {
		match self {
			ValueType::Ref(r) => r.read_rest(reader),
			_ => Ok(()),
		}
	}

	fn write_rest(&self, writer: &mut impl io::Write) -> Result<(), Error> {
		match self {
			ValueType::Ref(r) => r.write_rest(writer),
			_ => Ok(()),
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
	/// Reference to a specific definition
	Ref(u32),
}

impl RefType {
	fn from_bits(x: i8) -> Option<RefType> {
		match x {
			ANYFUNCTYPE => Some(RefType::AnyFunc),
			ANYREFTYPE => Some(RefType::AnyRef),
			REFTYPE => Some(RefType::Ref(0)),
			_ => None,
		}
	}

	fn to_bits(self) -> i8 {
		match self {
			RefType::AnyFunc => ANYFUNCTYPE,
			RefType::AnyRef => ANYREFTYPE,
			RefType::Ref(_) => REFTYPE,
		}
	}

	fn read_rest(&mut self, reader: &mut impl io::Read) -> Result<(), Error> {
		match self {
			RefType::Ref(i) => *i = VarUint32::deserialize(reader)?.into(),
			_ => (),
		};
		Ok(())
	}

	fn write_rest(&self, writer: &mut impl io::Write) -> Result<(), Error> {
		match self {
			RefType::Ref(i) => VarUint32::from(*i).serialize(writer),
			_ => Ok(()),
		}
	}
}

impl Deserialize for RefType {
	type Error = Error;

	fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
		let val = VarInt7::deserialize(reader)?;
		let mut item = RefType::from_bits(val.into())
			.ok_or(Error::UnknownValueType(val.into()))?;
		item.read_rest(reader)?;
		Ok(item)
	}
}

impl Serialize for RefType {
	type Error = Error;

	fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
		let val: VarInt7 = self.to_bits().into();
		val.serialize(writer)?;
		self.write_rest(writer)?;
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
			I32TYPE => Some(NumType::I32),
			I64TYPE => Some(NumType::I64),
			F32TYPE => Some(NumType::F32),
			F64TYPE => Some(NumType::F64),
			_ => None,
		}
	}

	fn to_bits(self) -> i8 {
		match self {
			NumType::I32 => I32TYPE,
			NumType::I64 => I64TYPE,
			NumType::F32 => F32TYPE,
			NumType::F64 => F64TYPE,
		}
	}
}

impl Deserialize for ValueType {
	type Error = Error;

	fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
		let val = VarInt7::deserialize(reader)?;
		let mut item = ValueType::from_bits(val.into())
			.ok_or(Error::UnknownValueType(val.into()))?;
		item.read_rest(reader)?;
		Ok(item)
	}
}

impl Serialize for ValueType {
	type Error = Error;

	fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
		let val: VarInt7 = self.to_bits().into();
		val.serialize(writer)?;
		self.write_rest(writer)?;
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
			ValueType::Ref(RefType::Ref(idx)) => write!(f, "(ref {})", idx),
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
			NORESULTTYPE => Some(BlockType::NoResult),
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
			BlockType::NoResult => NORESULTTYPE,
			BlockType::Value(v) => v.to_bits(),
		}.into();
		val.serialize(writer)?;
		Ok(())
	}
}

/// Function signature type.
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct FunctionType {
	params: Vec<ValueType>,
	return_type: Option<ValueType>,
}

impl Default for FunctionType {
	fn default() -> Self {
		FunctionType {
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
			params: params,
			return_type: return_type,
		})
	}
}

impl Serialize for FunctionType {
	type Error = Error;

	fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
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

/// Structure type.
#[derive(Clone, Copy, Debug, PartialEq, Hash, Eq)]
pub enum StorageType {
	Value(ValueType),
	PackedI8,
	PackedI16,
}

impl Deserialize for StorageType {
	type Error = Error;

	fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
		let val: i8 = VarInt7::deserialize(reader)?.into();
		match val {
			PACKEDI8TYPE => Some(StorageType::PackedI8),
			PACKEDI16TYPE => Some(StorageType::PackedI16),
			_ => match ValueType::from_bits(val) {
				Some(mut item) => {
					item.read_rest(reader)?;
					Some(StorageType::Value(item))
				},
				None => None,
			},
		}.ok_or(Error::UnknownValueType(val))
	}
}

impl Serialize for StorageType {
	type Error = Error;

	fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
		match self {
			StorageType::PackedI8 => VarInt7::from(PACKEDI8TYPE).serialize(writer),
			StorageType::PackedI16 => VarInt7::from(PACKEDI16TYPE).serialize(writer),
			StorageType::Value(val) => {
				VarInt7::from(val.to_bits()).serialize(writer)?;
				val.write_rest(writer)
			},
		}
	}
}

/// Field type.
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct FieldType {
	elem: StorageType,
	mutable: bool,
}

impl Deserialize for FieldType {
	type Error = Error;

	fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
		let mutable = VarUint1::deserialize(reader)?.into();
		Ok(FieldType {
			elem: StorageType::deserialize(reader)?,
			mutable,
		})
	}
}

impl Serialize for FieldType {
	type Error = Error;

	fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
		VarUint1::from(self.mutable).serialize(writer)?;
		self.elem.serialize(writer)?;
		Ok(())
	}
}

/// Structure type.
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct StructType {
	fields: Vec<FieldType>,
}

impl Deserialize for StructType {
	type Error = Error;

	fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
		let fields: Vec<FieldType> = CountedList::deserialize(reader)?.into_inner();
		Ok(StructType {
			fields,
		})
	}
}

impl Serialize for StructType {
	type Error = Error;

	fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
		CountedListWriter::<FieldType, _>(
			self.fields.len(),
			self.fields.into_iter(),
		).serialize(writer)
	}
}

/// Array type.
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub struct ArrayType {
	elem: FieldType,
}

impl Deserialize for ArrayType {
	type Error = Error;

	fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
		Ok(ArrayType {
			elem: FieldType::deserialize(reader)?,
		})
	}
}

impl Serialize for ArrayType {
	type Error = Error;

	fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
		self.elem.serialize(writer)
	}
}
