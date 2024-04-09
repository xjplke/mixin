#[cfg(test)]
mod tests {
    use mixinx::{declare, expand, insert};
    use serde::{Deserialize, Serialize};

    //将person结构注册到mixin
    #[declare]
    #[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
    pub struct Person {
        pub name: String,
        pub age: i32,
    }

    //将person的impl注册到mixin

    #[expand]
    impl Person {
        pub fn print(&self) {
            //这里没有Person对象，包装类中如果没有各个方法，则实际会调用这个方法，且self是包装对象。
            println!("{:?}", self); //因为这里的self是包装对象，所以需要包装对象
        }
    }

    //Student  mixin了Person的属性及方法，同时将student的结构也注册到mixin，方便其他对象进行mixin。然后如果有同名的属性会使用自身的字段。
    #[insert(Person)]
    #[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
    pub struct Student {
        pub name: String,
        pub school: String,
        pub school_addr: String,
    }

    #[test]
    fn test_mixin() {
        let mut s = Student {
            age: 25,
            name: "aaaa".into(),
            school: "BJU".into(),
            school_addr: "Beijin".into(),
        };
        println!("Person info {:?}", s.get_person());
        s.print();

        let p = s.get_person();
        assert_eq!(
            p,
            Person {
                name: "aaaa".into(),
                age: 25,
            }
        );
        p.print();

        let np = Person {
            name: "bbbb".into(),
            age: 30,
        };
        s.set_person(&np);

        assert_eq!(s.get_person(), np);

        let s_str = serde_json::to_string(&s).unwrap();
        println!("s_str = {}", s_str);
    }
}
