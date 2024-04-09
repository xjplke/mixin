#[cfg(test)]
mod tests {

    use mixin::{declare, expand, insert, overwrite};

    trait Handler<T> {
        fn handle(&self, t: T) -> String;
    }

    struct Online {}
    struct Offline {}

    #[declare]
    pub struct Device {
        pub name: String,
    }

    #[expand]
    impl Device {
        pub fn get_name(&self) -> &String {
            &self.name
        }
        pub fn set_name(&mut self, name: String) {
            self.name = name;
        }
    }

    #[expand]
    impl Handler<Online> for Device {
        fn handle(&self, _t: Online) -> String {
            format!("Device Handle Online {}", self.name)
        }
    }

    #[expand]
    impl Handler<Offline> for Device {
        fn handle(&self, _t: Offline) -> String {
            format!("Device Handle Offline {}", self.name)
        }
    }

    fn handle_online(handler: &impl Handler<Online>) -> String {
        handler.handle(Online {})
    }

    fn handle_offline(handler: &impl Handler<Offline>) -> String {
        handler.handle(Offline {})
    }

    #[overwrite]
    impl APDevice {
        pub fn get_name(&self) -> &String {
            &self.name
        }
    }
    //这里有个问题， 如果想覆盖原来的方法怎么半？ 因为这个impl是没有注册，没办法给inset宏观察到的。
    //这里添加expand_trait微博能行，因为这个expand_trait的执行是在insert[Device]的后面
    //这个需要编译器自身把上下文也暴露出来，能够被观察到了之后，才能进行相关的处理吧？
    //如果把代码位置放在APDevice前面能，词法分析会先进行吗？->可以的，但是代码的处理流程又不同了。开一个新宏来处理这种情况
    #[overwrite]
    impl Handler<Online> for APDevice {
        fn handle(&self, _t: Online) -> String {
            println!("{}", self.get_device().handle(_t)); //这里也可以模拟调用"父对象"的方法。
            format!("APDevice Handle Online {}", self.name)
        }
    }

    #[insert(Device)]
    pub struct APDevice {}

    #[test]
    fn test_mixin() {
        let mut d = Device {
            name: "Device".into(),
        };

        let ap = APDevice {
            name: "APDevice".into(),
        };

        assert_eq!(handle_online(&d), "Device Handle Online Device");
        assert_eq!(handle_offline(&d), "Device Handle Offline Device");

        assert_eq!(handle_online(&ap), "APDevice Handle Online APDevice"); //APDevice Over Write handle
        assert_eq!(handle_offline(&ap), "Device Handle Offline APDevice");

        let d_new: String = "DeviceNewName".into();
        d.set_name(d_new.clone());
        assert_eq!(&d_new, d.get_name());
    }
}
