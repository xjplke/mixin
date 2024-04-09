//use caller_modpath::CallerModpath;
use convert_case::{Case, Casing};
use once_cell::sync::Lazy;
use proc_macro::{TokenStream, TokenTree};
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::quote;
use quote::ToTokens;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::DataStruct;
use syn::FieldsNamed;
use syn::ImplItem;
use syn::ImplItemFn;
use syn::Variant;
use syn::{Data, DeriveInput, Field, Fields, ItemImpl, Type, TypePath};
use thiserror::Error;

#[derive(Debug, Eq, PartialEq, Clone)]
enum MixinType {
    Unknown,
    Enum,
    Struct,
}

#[derive(Error, Debug)]
enum Error {
    #[error("global data unavailable")]
    GlobalUnavailable,
    #[error("can't find mixin with name: {0}")]
    NoMixin(String),
    #[error("invalid expansion of the mixin")]
    InvalidExpansion,
    #[error("syn error: {0}")]
    SynError(#[from] syn::Error),
    #[error("lex error: {0}")]
    LexError(#[from] proc_macro::LexError),
    #[error("parameter error")]
    ParameterError,
    #[error("You SHOULD place overwrite for {0} before insert")]
    OverWriteError(String),
    #[error("You impl trait {0} twice")]
    OverWriteTraitTwice(String),
    //#[error("Struct should be not declear more one")]
    //StructDeclearMoreOne,
    #[error("Unsupport Type {0}")]
    UnsupportType(String),
}

impl Error {
    fn to_compile_error(self) -> TokenStream {
        let txt = self.to_string();
        let err = syn::Error::new(Span::call_site(), txt).to_compile_error();
        TokenStream::from(err)
    }
}

//如果有多个impl Struct，是可以的
//如果有多个impl Trait for Struct是不可以有相同大的Trait出现的。
//所以impl Trait for Struct需要记录Trait的信息。
//因为insert的Struct也会建立对应的Mixin，但是如果有overwrite的 impl的话，其mixin是在inser之前建立的。
//需要解析出impl中具体的函数吗？
//因为overwrite的存在，所以insert中的流程应该是
//1、从全局变量中获取the_struct的mixin，如果有，说明其中有overwrite，但是这个overwrite的trait

#[proc_macro_attribute]
pub fn insert(args: TokenStream, input: TokenStream) -> TokenStream {
    //dbg!(input.to_string());
    insert_impl(args, input).unwrap_or_else(Error::to_compile_error)
}

fn insert_impl(args: TokenStream, input: TokenStream) -> Result<TokenStream, Error> {
    let mut output: TokenStream = "#[allow(dead_code)]".parse()?;
    output.extend(input.clone().into_iter());
    let mut data = GLOBAL_DATA.lock().map_err(|_| Error::GlobalUnavailable)?;
    let the_struct: DeriveInput = syn::parse(input.clone())?;
    let the_struct_name = the_struct.ident.to_string();
    //    dbg!(&the_struct_name);
    let the_struct_mixin = data.get(&the_struct_name); //如果这里有值，说明有overwrite的处理
                                                       //dbg!(&the_struct_mixin);
    let mut the_struct_mixin_ctx = if let Some(mixin) = the_struct_mixin {
        let mut ctx = MixinCtx::from(mixin);
        ctx.declaration = Some(the_struct);
        ctx
    } else {
        let mixin_type = match the_struct.data {
            Data::Struct(_) => MixinType::Struct,
            Data::Enum(_) => MixinType::Enum,
            Data::Union(_) => todo!(),
        };
        MixinCtx {
            name: the_struct.ident.clone(),
            mixin_type,
            declaration: Some(the_struct),
            extensions: HashMap::new(),
            overwrite_impls: HashMap::new(),
            impl_traits: HashMap::new(),
            over_traits: HashMap::new(),
        }
    };

    // Get names of mixins to append
    let mut mixin_names = HashSet::new();
    for ident in args.into_iter() {
        //需要mixin的结构名称是从参数来的，如果是跨包的mixin，这里是不是需要用包的全路径才行？
        if let TokenTree::Ident(idt) = ident {
            mixin_names.insert(idt.to_string());
        }
    }
    //dbg!(&mixin_names);

    //如果mixin_type 是struct，需要混入字段。
    let mut mixed_fields = Vec::new();
    let mut mixed_variants = Vec::new();
    for mixin_name in mixin_names {
        let mixin = data
            .get(&mixin_name)
            .ok_or_else(|| Error::NoMixin(mixin_name.clone()))?; //根据mixin_name从全局变量中找到对应的mixin

        let extend_mixin_ctx: MixinCtx = mixin.into();
        //extend_mixin_ctx.dbg_print();
        //dbg!(&extend_mixin_ctx.declaration);
        //将mixin的field 汇总
        if let Data::Struct(st) = extend_mixin_ctx.declaration.clone().unwrap().data {
            if let Fields::Named(named) = st.fields {
                mixed_fields.push(named.named); //先把mixin的field push到mixed_fields, 后面将这些field输出到the_struct的field
            }
        } else if let Data::Enum(en) = extend_mixin_ctx.declaration.unwrap().data {
            for variant in en.variants {
                mixed_variants.push(variant);
            }
        }
        //直接添加the_struct_mixin_ctx中的fn，如果
        for (fn_name, fn_impl) in extend_mixin_ctx.extensions.iter() {
            the_struct_mixin_ctx
                .extensions
                .insert(fn_name.clone(), fn_impl.clone());
        }

        //直接用overwrite的内容覆盖，这里其实有点小问题(也不算问题)：原来没有写impl但是有overwrite的函数也会直接添加进去。
        //最理想的状态是能够提示overwrite的函数实际之前没有。
        // dbg!(&mixin_name);
        // for (fn_name, fn_impl) in extend_mixin_ctx.overwrite_impls.iter() {
        //     dbg!(&fn_name);
        //     the_struct_mixin_ctx
        //         .extensions
        //         .insert(fn_name.clone(), fn_impl.clone());
        // }

        //直接添加the_struct_mixin_ctx的trait，
        for (trait_name, trait_impl) in extend_mixin_ctx.impl_traits.iter() {
            //因为这里是ItemImpl, 需要其中的self_ty,再插入到the_struct_mixin_ctx
            let mut trait_impl = trait_impl.clone();

            let ty = trait_impl.self_ty.as_mut(); //这里的self_ty一定是Struct的Type，也就是for后面的值，我们需要将其中的ident替换成目标struct
            let path = if let Type::Path(TypePath { path, .. }, ..) = ty {
                path
            } else {
                return Err(Error::UnsupportType(ty.into_token_stream().to_string()));
            };
            let x = path.segments.last_mut().unwrap();
            x.ident = Ident::new(&the_struct_name, x.ident.span());

            the_struct_mixin_ctx
                .impl_traits
                .insert(trait_name.clone(), trait_impl);
        }
        // //然后再用overwrite的trait去覆盖
        // for (trait_name, trait_impl) in the_struct_mixin_ctx.over_traits.iter() {
        //     the_struct_mixin_ctx
        //         .impl_traits
        //         .insert(trait_name.clone(), trait_impl.clone());
        // }
    }

    //overwrite 是在将自己mixin中的overwrite，在insert其他minxin之后进行覆盖。之前的代码逻辑是错误的。
    for (fn_name, fn_impl) in the_struct_mixin_ctx.overwrite_impls.iter() {
        //        dbg!(&fn_name);
        the_struct_mixin_ctx
            .extensions
            .insert(fn_name.clone(), fn_impl.clone());
    }
    for (trait_name, trait_impl) in the_struct_mixin_ctx.over_traits.iter() {
        the_struct_mixin_ctx
            .impl_traits
            .insert(trait_name.clone(), trait_impl.clone());
    }

    //if let Data::Struct(ref mut st) = the_struct.data {
    //直接修改the_struct_mixin_ctx的declareation
    if let Data::Struct(ref mut st) = the_struct_mixin_ctx.declaration.as_mut().unwrap().data {
        if let Fields::Named(ref mut named) = st.fields {
            let mut the_struct_fields = HashSet::<String>::new();
            for p in named.named.iter() {
                if let Some(idt) = p.ident.clone() {
                    the_struct_fields.insert(idt.to_string());
                }
            } //将自己的field保存到一个hashset中

            //遍历mixin的field，并添加到new_fields中，跳过自己已经有的字段
            for fields in mixed_fields {
                let mut new_fields: Punctuated<Field, Comma> = Punctuated::new();
                for field in fields.iter() {
                    if let Some(idt) = field.ident.clone() {
                        //如果存在同名的field则跳过
                        if !the_struct_fields.contains(&idt.to_string()) {
                            //同时把添加的filed push到the_struct_fields，避免多个mixin中有相同的filed导致最后有问题。
                            the_struct_fields.insert(idt.to_string());
                            new_fields.push(field.clone());
                        }
                    }
                }
                //把new fields添加到最终的输出。
                named.named.extend(new_fields.into_pairs());
            }
        }
    } else if let Data::Enum(ref mut en) = the_struct_mixin_ctx.declaration.as_mut().unwrap().data {
        //这里的代码感觉有点乱， 考虑重构的时候讲不同类型的declare，insert、拆分成不用的实现。
        let mut the_enum_varients = HashSet::new();
        for variant in en.variants.iter() {
            the_enum_varients.insert(variant.ident.clone().to_string());
        }
        //dbg!(&the_enum_varients);
        let mut new_variants: Punctuated<Variant, Comma> = Punctuated::new();
        for variant in mixed_variants {
            if !the_enum_varients.contains(&variant.ident.clone().to_string()) {
                the_enum_varients.insert(variant.ident.clone().to_string());
                new_variants.push(variant);
            }
        }
        en.variants.extend(new_variants.into_pairs());
    }

    //添加get_set方法
    let declaration = the_struct_mixin_ctx.declaration.as_ref().unwrap();
    //只有类型是Struct时，才需要生产 get_set方法。
    if let Data::Struct(_) = declaration.data {
        let get_set_impls_stream = gen_get_set_impls(declaration);
        let get_set_impls = syn::parse::<ItemImpl>(get_set_impls_stream.into()).unwrap();
        the_struct_mixin_ctx.add_extension(&get_set_impls);
    }

    //这里的into_token_stream返回的是proc_macro2下的TokenStream,使用into转换成proc_macro下的
    let stream: TokenStream = the_struct_mixin_ctx.to_token_stream();
    let the_struct_mixin = Mixin::from(&the_struct_mixin_ctx);

    //the_struct_mixin_ctx.dbg_print();
    //dbg!(stream.to_string());
    //最后把the_struct_mixin放到全局变量， 这里实际会替换原来已经添加了overwrite的mixin。然后overwrite的信息已经没有用了。
    data.insert(the_struct_name, the_struct_mixin);
    Ok(stream)
}

#[derive(Debug)]
struct Mixin {
    name: String,
    mixin_type: MixinType,
    declaration: Option<String>, //这里是struct结构体的声明，直接用String类型，相当于是源码。
    //不用TokenStream或者DeriveInput是因为DeriveInput不能跨线程使用。
    extensions: Vec<String>, //这里是struct所有impl的声明，也是用String，相当于直接保存的源码。
    overwrite_impls: HashMap<String, String>, //key 是fn的name，string是fn的源码，
    impl_traits: HashMap<String, String>, //key 的string是trait name, val的string是源码。
    over_traits: HashMap<String, String>,
}

struct MixinCtx {
    name: Ident,
    mixin_type: MixinType,
    declaration: Option<DeriveInput>,
    extensions: HashMap<String, ImplItemFn>, //
    overwrite_impls: HashMap<String, ImplItemFn>,
    impl_traits: HashMap<String, ItemImpl>, //key的String是trait name
    over_traits: HashMap<String, ItemImpl>,
}

fn insert_impl_hm(hm: &mut HashMap<String, ImplItemFn>, item_impl: &ItemImpl) {
    for impl_item in item_impl.items.iter() {
        match impl_item {
            ImplItem::Const(_) => todo!(),
            ImplItem::Fn(impl_item_fn) => {
                let ident_name = impl_item_fn.sig.ident.to_string();
                let pre = hm.get(&ident_name);
                if pre.is_some() {
                    //在overwrite里面有出现了重复的函数？？？这种允许吗？
                }
                hm.insert(ident_name, impl_item_fn.clone()); //
            }
            ImplItem::Type(_) => todo!(),
            ImplItem::Macro(_) => todo!(),
            ImplItem::Verbatim(_) => todo!(),
            _ => todo!(),
        }
    }
}

impl MixinCtx {
    #[allow(dead_code)]
    fn dbg_print(&self) {
        let mixin_name = self.name.to_string();
        dbg!("=========================", mixin_name);
        dbg!(self.declaration.to_token_stream().to_string());
        for item_fn in self.extensions.iter() {
            dbg!(item_fn.0, item_fn.1.to_token_stream().to_string());
        }
        for item_impl in self.impl_traits.iter() {
            dbg!(item_impl.0, item_impl.1.to_token_stream().to_string());
        }
    }

    fn add_overwrite_impls(&mut self, item_impl: &ItemImpl) {
        insert_impl_hm(&mut self.overwrite_impls, item_impl)
    }
    fn add_extension(&mut self, item_impl: &ItemImpl) {
        insert_impl_hm(&mut self.extensions, item_impl)
    }

    fn to_token_stream(&self) -> TokenStream {
        if self.declaration.is_none() {
            return Error::InvalidExpansion.to_compile_error();
        }
        let name = self.name.clone();
        let derive_input = self.declaration.clone().unwrap();

        //for impl fn
        let impl_fns_token: Vec<TokenStream2> = self
            .extensions
            .iter()
            .map(|(_, impl_fn)| quote! { #impl_fn })
            .collect();
        //https://docs.rs/syn/latest/syn/struct.Generics.html
        let (impl_generics, ty_generics, where_clause) = derive_input.generics.split_for_impl();
        let impl_token = quote! {
            impl #impl_generics #name #ty_generics #where_clause{
                #(#impl_fns_token)*
            }
        };

        let mut stream: TokenStream2 = derive_input.clone().into_token_stream();
        stream.extend(impl_token);

        //for impl trait
        for (_, trait_impl) in self.impl_traits.iter() {
            stream.extend(trait_impl.to_token_stream())
        }
        stream.into()
    }
}

impl From<&Mixin> for MixinCtx {
    fn from(value: &Mixin) -> Self {
        let name = Ident::new(&value.name, Span::call_site());
        let declaration = if let Some(declaration) = value.declaration.as_ref() {
            Some(syn::parse::<DeriveInput>(declaration.parse::<TokenStream>().unwrap()).unwrap())
        } else {
            None
        };

        let mut extensions = HashMap::new();
        for extention in value.extensions.iter() {
            let ext_tokenstream = extention.parse::<TokenStream>().unwrap();
            let ext_item_impl = syn::parse(ext_tokenstream).unwrap();
            insert_impl_hm(&mut extensions, &ext_item_impl);
        }

        let mut overwrite_impls = HashMap::new();
        for (_, overwrite_impl) in value.overwrite_impls.iter() {
            let ov_tokenstream = overwrite_impl.parse::<TokenStream>().unwrap();
            let ov_item_impl: ImplItemFn = syn::parse(ov_tokenstream).unwrap();
            overwrite_impls.insert(ov_item_impl.sig.ident.to_string(), ov_item_impl);
        }
        let mut impl_traits = HashMap::new();
        for ov in value.impl_traits.iter() {
            let trait_impl = syn::parse::<ItemImpl>(ov.1.parse().unwrap()).unwrap();
            impl_traits.insert(ov.0.clone(), trait_impl);
        }

        let mut over_traits = HashMap::new();
        for ov in value.over_traits.iter() {
            let ov_trait = syn::parse::<ItemImpl>(ov.1.parse().unwrap()).unwrap();
            over_traits.insert(ov.0.clone(), ov_trait);
        }

        MixinCtx {
            name,
            mixin_type: value.mixin_type.clone(),
            declaration: declaration,
            extensions: extensions,
            overwrite_impls: overwrite_impls,
            impl_traits: impl_traits,
            over_traits: over_traits,
        }
    }
}

impl From<&MixinCtx> for Mixin {
    fn from(value: &MixinCtx) -> Self {
        let name: Ident = value.name.clone();

        let declaration = if let Some(declaration) = value.declaration.as_ref() {
            Some(declaration.to_token_stream().to_string())
        } else {
            None
        };

        //for impl fn
        let mut extensions: Vec<String> = Vec::new(); //其实这里放Vec没啥意义，因为MixinCtx中会合并多个impl
        let impl_fns_token: Vec<TokenStream2> = value
            .extensions
            .iter()
            .map(|(_, impl_fn)| quote! { #impl_fn })
            .collect();
        let impl_token = quote! {
            impl #name {
                #(#impl_fns_token)*
            }
        };
        extensions.push(impl_token.to_string());

        //for overwrite_impls
        let mut overwrite_impls = HashMap::new();
        for trait_impl in value.overwrite_impls.iter() {
            overwrite_impls.insert(
                trait_impl.0.clone(),
                trait_impl.1.to_token_stream().to_string(),
            );
        }

        //for impl trait
        let mut impl_traits = HashMap::new();
        for (trait_name, trait_impl) in value.impl_traits.iter() {
            impl_traits.insert(trait_name.clone(), trait_impl.to_token_stream().to_string());
        }

        //for over_traits
        let mut over_traits = HashMap::new();
        for (trait_name, trait_impl) in value.over_traits.iter() {
            over_traits.insert(trait_name.clone(), trait_impl.to_token_stream().to_string());
        }

        Mixin {
            name: name.to_string(),
            mixin_type: value.mixin_type.clone(),
            declaration: declaration,
            extensions: extensions,
            overwrite_impls: overwrite_impls,
            impl_traits: impl_traits,
            over_traits: over_traits,
        }
    }
}
//全局变量。通过declare和expand将对应的结构的声明以及impl实现保存起来，然后在insert的时候，将其添加到另外struct的源码上。
static GLOBAL_DATA: Lazy<Mutex<HashMap<String, Mixin>>> = Lazy::new(|| Mutex::new(HashMap::new()));

fn gen_get_set_impls(input: &DeriveInput) -> TokenStream2 {
    let name = &input.ident;

    let name_string = name.to_string();
    let get_fn_name = Ident::new(
        &("get".to_owned() + &name_string).to_case(Case::Snake),
        name.span(),
    );
    let set_fn_name = Ident::new(
        &("set".to_owned() + &name_string).to_case(Case::Snake),
        name.span(),
    );

    let fields = if let Data::Struct(DataStruct {
        fields: Fields::Named(FieldsNamed { named, .. }),
        ..
    }) = &input.data
    {
        named
    } else {
        panic!("Unsupported data type");
    };
    let fds: Vec<Ident> = fields
        .into_iter()
        .map(|f| f.ident.clone().unwrap())
        .collect();
    let get_fds_token: Vec<TokenStream2> = fds
        .iter()
        .map(|name| quote! { #name: self.#name.clone() })
        .collect();
    let set_fds_token: Vec<TokenStream2> = fds
        .iter()
        .map(|name| quote! { self.#name = p.#name.clone() })
        .collect();
    //generate get/set functions
    //https://docs.rs/syn/latest/syn/struct.Generics.html
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let impl_get_set = quote! {
        impl #impl_generics #name #ty_generics #where_clause {
            pub fn #get_fn_name(&self) -> #name #ty_generics{
                #name {
                    #(#get_fds_token,)*
                }
            }
            pub fn #set_fn_name(&mut self, p: &#name #ty_generics){
                #(#set_fds_token;)*
            }
        }
    };
    //dbg!(impl_get_set.to_token_stream().to_string());
    impl_get_set
}

#[proc_macro_attribute]
pub fn declare(_attribute: TokenStream, input: TokenStream) -> TokenStream {
    declare_impl(input).unwrap_or_else(Error::to_compile_error)
}

fn declare_impl(input: TokenStream) -> Result<TokenStream, Error> {
    // Keep it just to let the compiler check it
    let mut output: TokenStream = "#[allow(dead_code)]".parse()?;
    output.extend(input.clone().into_iter());

    let input = syn::parse::<DeriveInput>(input).unwrap();
    let mixin_type = match input.data {
        Data::Struct(_) => MixinType::Struct,
        Data::Enum(_) => MixinType::Enum,
        Data::Union(_) => todo!(),
    };

    let name_string = input.ident.clone().to_string();

    let mut get_set_impls = None;

    if mixin_type == MixinType::Struct {
        let get_set_impls_stream = gen_get_set_impls(&input);
        get_set_impls = Some(syn::parse::<ItemImpl>(get_set_impls_stream.into())?);
    }

    let mut mixin_ctx = MixinCtx {
        name: input.ident.clone(),
        mixin_type: mixin_type.clone(),
        declaration: Some(input),
        extensions: HashMap::new(),
        overwrite_impls: HashMap::new(),
        impl_traits: HashMap::new(),
        over_traits: HashMap::new(),
    };

    if mixin_type == MixinType::Struct {
        mixin_ctx.add_extension(&get_set_impls.unwrap());
    }

    let mixin = (&mixin_ctx).into();
    let mut data: std::sync::MutexGuard<'_, HashMap<String, Mixin>> =
        GLOBAL_DATA.lock().map_err(|_| Error::GlobalUnavailable)?;
    data.insert(name_string, mixin);
    Ok(mixin_ctx.to_token_stream())
}

#[proc_macro_attribute]
pub fn expand(_attribute: TokenStream, input: TokenStream) -> TokenStream {
    expand_impl(input).unwrap_or_else(Error::to_compile_error)
}

//获得impl的name以及trait_name, 如果不是trait，则trait_name返回空字符串
fn get_name_of_impl(input: &ItemImpl) -> Result<(String, String), Error> {
    let trait_name = if let Some((_, path, _)) = &input.trait_ {
        path.to_token_stream().to_string()
    } else {
        "".into()
    };

    let ty = input.self_ty.as_ref();
    let path = if let Type::Path(TypePath { path, .. }, ..) = ty {
        path
    } else {
        return Err(Error::UnsupportType(ty.to_token_stream().to_string()));
    };

    let idt = path.get_ident().unwrap();
    let name = idt.to_string();

    Ok((name, trait_name))
}

fn expand_impl(input: TokenStream) -> Result<TokenStream, Error> {
    let input = syn::parse::<ItemImpl>(input).unwrap();
    let output = input.to_token_stream().into();

    let (name, trait_name) = get_name_of_impl(&input)?;

    let mut data = GLOBAL_DATA.lock().map_err(|_| Error::GlobalUnavailable)?;
    let mixin = data
        .get(&name)
        .ok_or_else(|| Error::NoMixin(name.clone()))?; //extend不能放在结构体declear的前面。

    //let mut mixin_ctx = MixinCtx::from(mixin);

    let mut mixin_ctx: MixinCtx = mixin.into();

    if trait_name != String::from("") {
        mixin_ctx.impl_traits.insert(trait_name, input);
    } else {
        mixin_ctx.add_extension(&input);
    }

    let mixin: Mixin = (&mixin_ctx).into();
    data.insert(name, mixin);

    Ok(output)
}

//#[caller_modpath::expose_caller_modpath]
#[proc_macro_attribute]
pub fn overwrite(attribute: TokenStream, input: TokenStream) -> TokenStream {
    if !attribute.is_empty() {
        let e = Error::ParameterError;
        return e.to_compile_error();
    }

    overwrite_impl(input).unwrap_or_else(Error::to_compile_error)
}

fn overwrite_impl(input: TokenStream) -> Result<TokenStream, Error> {
    let input = syn::parse::<ItemImpl>(input).unwrap();

    let (name, trait_name) = get_name_of_impl(&input)?;

    let mut data = GLOBAL_DATA.lock().map_err(|_| Error::GlobalUnavailable)?;

    let mut mixin_ctx = if let Some(mixin) = data.get(&name) {
        MixinCtx::from(mixin)
    } else {
        //没有找到就新建一个并且放到全局变量,这个时候还不知道当前结构是 Struct还是enum。
        MixinCtx {
            name: Ident::new(&name, Span::call_site()),
            mixin_type: MixinType::Unknown,
            declaration: None,
            extensions: HashMap::new(),
            overwrite_impls: HashMap::new(),
            impl_traits: HashMap::new(),
            over_traits: HashMap::new(),
        }
    };

    if mixin_ctx.declaration.is_some() {
        //如果这里先找到了mixin，并且declaration有值， 说明当前的这个overwrite没放到insert之前
        return Err(Error::OverWriteError(name));
    }

    if trait_name == String::from("") {
        //当前是不带trait的overwrie，需要把函数拆解出来
        mixin_ctx.add_overwrite_impls(&input);
    } else {
        if mixin_ctx.over_traits.contains_key(&trait_name) {
            return Err(Error::OverWriteTraitTwice(trait_name));
        }
        mixin_ctx.over_traits.insert(trait_name, input);
    }

    let mixin = Mixin::from(&mixin_ctx);
    data.insert(name, mixin);

    let output = "".parse::<TokenStream>().unwrap();
    Ok(output) //这里需要返回空的TokenStream，实际在的调用insert的时候再输出。
}
