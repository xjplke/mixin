#[cfg(test)]
mod tests {
    use crate::tests::{ap::APDevice, device::Device};
    trait Handler<T> {
        fn handle(&self, t: T) -> String;
    }

    pub mod device {
        use mixin::{declare, insert};
        use serde::{Deserialize, Serialize};

        #[declare]
        #[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
        pub struct DevicePersist {
            #[serde(rename(serialize = "Name", deserialize = "Name"))]
            pub name: String,
            pub cpu: i32,
        }

        #[insert(DevicePersist)]
        #[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
        pub struct Device {
            #[serde(rename(serialize = "Name", deserialize = "Name"))]
            pub name: String,
            #[serde(rename(serialize = "localtion_xxx", deserialize = "localtion_xxx"))]
            pub localtion: String,
        }
    }

    mod ap {
        use crate::tests::device::*;
        use mixin::insert;
        use serde::{Deserialize, Serialize};

        #[insert(DevicePersist)]
        #[derive(Debug, PartialEq, Eq)]
        pub struct APPersist {
            pub name: String,
            pub wlan: String,
        }

        //Device和APPersist中有重复的字段，需要在生成代码的时候避免重复的字段
        #[insert(Device, APPersist)]
        #[derive(Debug, PartialEq, Eq, Deserialize, Serialize)] //如果前面的类型添加了serde属性，insert的目标对象必须要实现Deserialize和Seialize
        pub struct APDevice<T: Clone> {
            pub client_num: u32,
            pub content: T,
        }
    }

    #[test]
    fn test_mixin() {
        let mut d = Device {
            name: "Device".into(),
            cpu: 1,
            localtion: "aaa".into(),
        };

        let mut ap: APDevice<String> = APDevice {
            name: "APDevice".into(),
            cpu: 2,
            client_num: 2,
            wlan: "abc".into(),
            localtion: "aaa".into(),
            content: "aaaa".into(),
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
