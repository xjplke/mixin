# mixin
mixin struct or enum

mixin not only struct fields, but also impl funcs and traits.

example 
'''
#[cfg(test)]
mod tests {
    use mixin::{declare, expand, insert};
    use serde::{Deserialize, Serialize};

    //use declare to register Person to mixin
    #[declare]
    #[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
    pub struct Person {
        pub name: String,
        pub age: i32,
    }

    //use "expand" to register impl for Person to mixin
    #[expand]
    impl Person {
        pub fn print(&self) {
            println!("{:?}", self);
        }
    }

    //use "insert" to mixin Person fields and methods, and Student is also registed to mixin.
    #[insert(Person)]
    #[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
    pub struct Student {
        pub name: String,
        pub school: String,
        pub school_addr: String,
    }

    //Employee mixin with Student，include the part of Person， and the filed 'name' cover 'name' in Student and Person。
    #[insert(Student)]
    #[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
    pub struct Employee {
        pub name: String,
        pub company: String,
        pub workage: i32,
    }

    #[test]
    fn test_mixin() {
        let mut e = Employee {
            company: "xjplke".into(),
            workage: 1,
            age: 25,
            name: "xjplke".into(),
            school: "BJU".into(),
            school_addr: "Beijin".into(),
        };
        e.print();
        println!("persion info {:?}", e.get_person());

        let p = e.get_person();
        assert_eq!(
            p,
            Person {
                name: "xjplke2".into(),
                age: 25,
            }
        );
        p.print();
        let s = e.get_student();
        assert_eq!(
            s,
            Student {
                age: 25,
                name: "xjplke3".into(),
                school: "BJU".into(),
                school_addr: "Beijin".into(),
            }
        );
        s.print();

        let sp = s.get_person();
        assert_eq!(p, sp);

        let np = Person {
            name: "xjplke4".into(),
            age: 30,
        };
        e.set_person(&np);

        assert_eq!(e.get_person(), np);

        let e_str = serde_json::to_string(&e).unwrap();
        println!("e_str = {}", e_str);
    }
}
'''

more examples is in tests