#[cfg(test)]
mod tests {
    use crate::tests::{ap::APDevice, device::Device};
    trait Handler<T> {
        fn handle(&self, t: T) -> String;
    }

    pub mod device {
        use mixin::{declare, insert};

        #[declare]
        #[derive(Debug, PartialEq, Eq)]
        pub struct DevicePersist {
            pub name: String,
            pub cpu: i32,
        }

        #[insert(DevicePersist)]
        #[derive(Debug, PartialEq, Eq)]
        pub struct Device {
            pub name: String,
            pub localtion: String,
        }
    }

    mod ap {
        use crate::tests::device::*;
        use mixin::insert;

        #[insert(DevicePersist)]
        #[derive(Debug, PartialEq, Eq)]
        pub struct APPersist {
            pub name: String,
            pub wlan: String,
        }

        //Device和APPersist中有重复的字段，需要在生成代码的时候避免重复的字段
        #[insert(Device, APPersist)]
        #[derive(Debug, PartialEq, Eq)]
        pub struct APDevice {
            pub client_num: u32,
        }
    }

    #[test]
    fn test_mixin() {
        let mut d = Device {
            name: "Device".into(),
            cpu: 1,
            localtion: "aaa".into(),
        };

        let mut ap = APDevice {
            name: "APDevice".into(),
            cpu: 2,
            client_num: 2,
            wlan: "abc".into(),
            localtion: "aaa".into(),
        };

        //println!("d = {:?}", d);
        //println!("ap = {:?}", ap);
        assert_ne!(ap.get_device(), d);
        ap.set_device(&d);
        assert_eq!(ap.get_device(), d);
        assert_eq!(ap.get_device_persist(), d.get_device_persist());

        let mut dp = d.get_device_persist();
        dp.cpu = 50;
        d.set_device_persist(&dp);
        assert_ne!(dp, ap.get_device_persist());
    }
}
