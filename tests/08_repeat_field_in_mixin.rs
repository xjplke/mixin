#[cfg(test)]
mod tests {
    use crate::tests::{ap::APDevice, device::Device};
    trait Handler<T> {
        fn handle(&self, t: T) -> String;
    }

    mod device {
        use mixinx::{declare, insert};

        #[declare]
        #[derive(Debug)]
        pub struct DevicePersist {
            pub name: String,
            pub cpu: i32,
        }

        #[insert(DevicePersist)]
        #[derive(Debug)]
        pub struct Device {
            pub name: String,
        }
    }

    mod ap {

        use crate::tests::device::*;
        use mixinx::insert;

        #[insert(DevicePersist)]
        #[derive(Debug)]
        pub struct APPersist {
            pub name: String,
            pub wlan: String,
        }

        //Device和APPersist中有重复的字段，需要在生成代码的时候避免重复的字段
        #[insert(Device, APPersist)]
        #[derive(Debug)]
        pub struct APDevice {
            pub client_num: u32,
        }
    }

    #[test]
    fn test_mixin() {
        let d = Device {
            name: "Device".into(),
            cpu: 1,
        };

        let ap = APDevice {
            name: "APDevice".into(),
            cpu: 2,
            client_num: 2,
            wlan: "abc".into(),
        };

        println!("d = {:?}", d);
        println!("ap = {:?}", ap);
    }
}
