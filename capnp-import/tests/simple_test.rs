#![allow(unused_imports)]

// Has to be top level
capnp_import::capnp_import!("tests/example.capnp");

use capnp::message;
use capnp::struct_list;

#[test]
fn simple_test() {
    use example_capnp::{date, person};
}

#[test]
fn test_anylist_passthrough() {
    use example_capnp::{date, test_generic};
    const NUM: u32 = 10;

    let mut message = message::Builder::new_default();
    {
        let root: test_generic::Builder<date::Owned> = message.init_root();
        let mut list: capnp::struct_list::Builder<'_, date::Owned> = root.init_any().initn_as(NUM);
        for idx in 0..NUM {
            let mut e = list.reborrow().get(idx);
            e.set_month(idx as u8);
            e.set_day((idx * 2) as u8);
            e.set_year((idx * 4 + 2000) as i16);
        }
    }

    {
        let root: test_generic::Reader<date::Owned> = message.get_root_as_reader().unwrap();
        let list: capnp::struct_list::Reader<'_, date::Owned> = root.get_any().get_as().unwrap();
        for idx in 0..NUM {
            let e = list.reborrow().get(idx);
            assert_eq!(e.get_month(), idx as u8);
            assert_eq!(e.get_day(), (idx * 2) as u8);
            assert_eq!(e.get_year(), (idx * 4 + 2000) as i16);
        }
    }

    {
        let root: test_generic::Builder<date::Owned> = message.get_root().unwrap();
        let mut list: capnp::struct_list::Builder<'_, date::Owned> =
            root.init_typed(1).initn_as(NUM);
        for idx in 0..NUM {
            let mut e = list.reborrow().get(idx);
            e.set_month(idx as u8);
            e.set_day((idx * 2) as u8);
            e.set_year((idx * 4 + 2000) as i16);
        }
    }

    {
        let root: test_generic::Reader<date::Owned> = message.get_root_as_reader().unwrap();
        let list: capnp::struct_list::Reader<'_, date::Owned> =
            root.get_typed().unwrap().get_as().unwrap();
        for idx in 0..NUM {
            let e = list.reborrow().get(idx);
            assert_eq!(e.get_month(), idx as u8);
            assert_eq!(e.get_day(), (idx * 2) as u8);
            assert_eq!(e.get_year(), (idx * 4 + 2000) as i16);
        }
    }

    let mut message2 = message::Builder::new_default();
    {
        let root: test_generic::Reader<date::Owned> = message.get_root_as_reader().unwrap();
        let mut root2: test_generic::Builder<date::Owned> = message2.init_root();
        root2.set_typed(root.get_any()).unwrap();

        let list2: capnp::struct_list::Reader<'_, date::Owned> =
            root2.get_typed().unwrap().into_reader().get_as().unwrap();
        for idx in 0..NUM {
            let e = list2.reborrow().get(idx);
            assert_eq!(e.get_month(), idx as u8);
            assert_eq!(e.get_day(), (idx * 2) as u8);
            assert_eq!(e.get_year(), (idx * 4 + 2000) as i16);
        }
    }

    //let b = TestImpl::new();
}

/*
struct TestImpl {
    dates: Vec<(i16, u8, u8)>,
}

impl<T: capnp::traits::Owned + capnp::traits::OwnedStruct> Server<T> for TestImpl {
    fn get_type_list(
        &mut self,
        _: GetTypeListParams<T>,
        mut results: GetTypeListResults<T>,
    ) -> Promise<(), ::capnp::Error> {
        let root = results.get().init_result(1);
        let mut list: capnp::struct_list::Builder<'_, example_capnp::date::Owned> =
            root.initn_as(10);
        for i in 0..self.dates.len() {
            let e = &mut list.reborrow().get(i as u32);
            e.set_year(self.dates[i].0);
            e.set_month(self.dates[i].1);
            e.set_day(self.dates[i].2);
        }

        Promise::ok(())
    }

    fn set_type_list(
        &mut self,
        params: SetTypeListParams<T>,
        _: SetTypeListResults<T>,
    ) -> Promise<(), ::capnp::Error> {
        let genlist = params.get().unwrap().get_result().unwrap();
        let datelist = genlist
            .get_as::<::capnp::struct_list::Reader<'_, example_capnp::date::Owned>>()
            .unwrap();
        self.dates.clear();
        for i in 0..datelist.len() {
            let date = datelist.get(i);
            self.dates
                .push((date.get_year(), date.get_month(), date.get_day()));
        }
        Promise::ok(())
    }

    fn get_any_list(
        &mut self,
        _: GetAnyListParams<T>,
        mut results: GetAnyListResults<T>,
    ) -> Promise<(), ::capnp::Error> {
        let root = results.get().init_result();
        let mut list: capnp::struct_list::Builder<'_, example_capnp::date::Owned> =
            root.initn_as(10);
        for i in 0..self.dates.len() {
            let e = &mut list.reborrow().get(i as u32);
            e.set_year(self.dates[i].0);
            e.set_month(self.dates[i].1);
            e.set_day(self.dates[i].2);
        }

        Promise::ok(())
    }

    fn set_any_list(
        &mut self,
        params: SetAnyListParams<T>,
        _: SetAnyListResults<T>,
    ) -> Promise<(), ::capnp::Error> {
        let genlist = params.get().unwrap().get_result();
        let datelist = genlist
            .get_as::<::capnp::struct_list::Reader<'_, example_capnp::date::Owned>>()
            .unwrap();
        self.dates.clear();
        for i in 0..datelist.len() {
            let date = datelist.get(i);
            self.dates
                .push((date.get_year(), date.get_month(), date.get_day()));
        }
        Promise::ok(())
    }
}
*/
