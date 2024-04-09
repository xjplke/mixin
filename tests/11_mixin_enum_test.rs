#[cfg(test)]
mod tests {
    use mixinx::{declare, expand, insert};

    #[declare]
    #[derive(Clone, Debug)]
    pub enum DeviceMessage {
        Register(String),
        Update(i32),
    }

    //#[expand]
    impl DeviceMessage {
        pub fn get_show(&self) -> String {
            match self {
                Self::Register(x) => x.to_string(),
                Self::Update(y) => y.to_string(),
            }
        }
    }
    #[expand]
    impl DeviceMessage {
        pub fn print(&self) {
            println!("{}", self.get_show());
        }
    }

    //#[overwrite]
    impl APMSG {
        fn get_show(&self) -> String {
            //这种基于具体枚举项目的实现需要overwrite才行。否则编译不过
            match self {
                Self::Register(x) => x.to_string(),
                Self::Reset(y) => y.to_string(),
                Self::Update(z) => z.to_string(),
            }
        }
    }
    #[insert(DeviceMessage)]
    #[derive(Clone, Debug)]
    pub enum APMSG {
        Register(String),
        Reset(i32),
    }

    #[test]
    fn test_mixin() {
        let dev_register = DeviceMessage::Register("dev_register".into());
        //println!("dev_msg info {:?}", dev_msg);
        assert_eq!("dev_register".to_owned(), dev_register.get_show());
        dev_register.print();

        let x = if let DeviceMessage::Register(y) = dev_register {
            y
        } else {
            "".into()
        };
        assert_eq!(x, "dev_register");
        let ap_msg_register = APMSG::Register(x);
        println!("ap_msg_register {:?}", ap_msg_register);
        ap_msg_register.print();

        let dev_update = DeviceMessage::Update(10);
        let z = if let DeviceMessage::Update(z) = dev_update {
            z
        } else {
            0
        };
        assert_eq!(z, 10);
        assert_eq!("10".to_owned(), dev_update.get_show());
        dev_update.print();

        let ap_update = APMSG::Update(10);
        let m = if let APMSG::Update(m) = ap_update {
            m
        } else {
            0
        };
        assert_eq!(m, 10);
        assert_eq!("10", ap_update.get_show());
        ap_update.print();

        let ap_reset = APMSG::Reset(100);
        assert_eq!("100", ap_reset.get_show());
        ap_reset.print();
    }
}
