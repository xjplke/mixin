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
            println!("{:?}", self);
        }
    }

    //Student  mixin了Person大属性及方法，同时将student的结构也注册到mixin，方便其他对象进行mixin。然后如果有同名的属性会使用自身的字段。
    #[declare]
    #[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
    pub struct Student {
        pub name: String, //person中有name，这里会忽略person中大的name
        pub school: String,
        pub school_addr: String,
    }

    //Employee mixin了Student，包括了Student mixin的Person的部分，以及Person/Student实现的方法。同名的name属性会覆盖。
    #[insert(Person, Student)]
    #[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
    pub struct Employee {
        pub name: String,
        pub company: String,
        pub workage: i32,
    }

    #[test]
    fn test_mixin() {
        let mut e = Employee {
            company: "xxx".into(),
            workage: 1,
            age: 25,
            name: "aaaa".into(),
            school: "BJU".into(),
            school_addr: "Beijin".into(),
        };
        e.print();
        println!("persion info {:?}", e.get_person());

        let p = e.get_person();
        assert_eq!(
            p,
            Person {
                name: "aaaa".into(),
                age: 25,
            }
        );
        p.print();

        let s = e.get_student();
        assert_eq!(
            s,
            Student {
                name: "aaaa".into(),
                school: "BJU".into(),
                school_addr: "Beijin".into(),
            }
        );

        let np = Person {
            name: "bbbb".into(),
            age: 30,
        };
        e.set_person(&np);

        assert_eq!(e.get_person(), np);

        let e_str = serde_json::to_string(&e).unwrap();
        println!("e_str = {}", e_str);
    }
}
