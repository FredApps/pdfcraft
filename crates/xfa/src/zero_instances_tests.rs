//! Synthetic occurrence regressions (XFA 3.3, chapter 7, “Repeating Elements using
//! Occurrence Limits”). No source document or viewer assets are used here.

use crate::layout::{Form, Item, Widget};
use crate::{layout, parse, parse_datasets};

fn widgets(form: &Form) -> Vec<&Widget> {
    form.pages
        .iter()
        .flat_map(|p| &p.items)
        .filter_map(|item| match item {
            Item::Widget(widget) => Some(widget.as_ref()),
            _ => None,
        })
        .collect()
}

fn render(body: &str, data: Option<&str>) -> Form {
    let (template, _) = parse(&format!(r#"<template><subform name="form" layout="tb" w="300pt">{body}</subform></template>"#)).unwrap();
    let data = data.map(|xml| {
        parse_datasets(&format!(r#"<xfa:datasets xmlns:xfa="http://www.xfa.org/schema/xfa-data/1.0/"><xfa:data>{xml}</xfa:data></xfa:datasets>"#))
            .unwrap()
    });
    layout(&template, data.as_ref()).unwrap()
}

#[test]
fn zero_instances_are_absent_in_every_container_layout() {
    for mode in ["position", "tb", "lr-tb", "rl-tb", "table"] {
        for height in ["", " h=\"80pt\""] {
            let child_mode = if mode == "table" { "row" } else { "tb" };
            let body = format!(
                r#"
                <subform name="box" layout="{mode}" w="300pt"{height}>
                    <subform name="optional" layout="{child_mode}" w="120pt" h="30pt">
                        <occur min="0" initial="0" max="1"/>
                        <field name="absent" w="100pt" h="20pt"><ui><textEdit/></ui></field>
                    </subform>
                    <field name="after" w="100pt" h="20pt"><ui><textEdit/></ui></field>
                </subform>
                <field name="following" w="100pt" h="20pt"><ui><textEdit/></ui></field>"#
            );
            let form = render(&body, None);
            let fields = widgets(&form);
            assert_eq!(fields.len(), 2, "mode={mode}, height={height}: {fields:?}");
            assert!(fields[0].som.ends_with("after[0]"));
            if height.is_empty() {
                assert!((fields[1].rect.y - fields[0].rect.y - 20.0).abs() < 0.01, "zero instance must take no height: {fields:?}");
            }
        }
    }
}

#[test]
fn occurrence_initial_is_for_blank_forms_and_minimum_for_data_merges() {
    let body = r#"<subform name="row" layout="tb" w="100pt" h="20pt">
        <occur min="0" initial="2" max="3"/>
        <field name="value" w="100pt" h="20pt"><ui><textEdit/></ui></field>
    </subform>"#;
    assert_eq!(widgets(&render(body, None)).len(), 2);
    assert_eq!(widgets(&render(body, Some(""))).len(), 2, "an empty data packet is a blank merge");
    assert_eq!(widgets(&render(body, Some("<form/>"))).len(), 0, "data contains no row");
    assert_eq!(widgets(&render(body, Some("<form><row/></form>"))).len(), 1);
    assert_eq!(widgets(&render(body, Some("<form><row/><row/><row/><row/></form>"))).len(), 3);
    let required = body.replace("min=\"0\"", "min=\"2\"");
    assert_eq!(widgets(&render(&required, Some("<form><row/></form>"))).len(), 2);
}

#[test]
fn optional_singleton_is_instantiated_when_its_data_exists() {
    let body = r#"<subform name="row" layout="tb"><occur min="0" initial="0" max="1"/>
        <field name="value" w="100pt" h="20pt"><ui><textEdit/></ui></field>
    </subform>"#;
    assert_eq!(widgets(&render(body, None)).len(), 0);
    assert_eq!(widgets(&render(body, Some("<form><row><value>present</value></row></form>"))).len(), 1);
    let prohibited = body.replace(" max=\"1\"", "");
    assert_eq!(widgets(&render(&prohibited, Some("<form><row/></form>"))).len(), 0, "omitted max defaults to min=0");
}

#[test]
fn zero_instances_take_no_height_and_do_not_trigger_page_breaks() {
    let body = r#"<field name="before" w="100pt" h="20pt"><ui><textEdit/></ui></field>
        <subform name="row" layout="tb"><occur min="0" initial="0" max="-1"/>
            <breakBefore targetType="pageArea"/><breakAfter targetType="pageArea"/>
            <field name="absent" w="100pt" h="200pt"><ui><textEdit/></ui></field>
        </subform>
        <field name="after" w="100pt" h="20pt"><ui><textEdit/></ui></field>"#;
    let form = render(body, None);
    assert_eq!(form.pages.len(), 1);
    let fields = widgets(&form);
    assert_eq!(fields.len(), 2);
    assert!((fields[1].rect.y - fields[0].rect.y - 20.0).abs() < 0.01);
}

#[test]
fn an_optional_subform_cell_uses_the_rows_data_scope() {
    let body = r#"<subform name="table" layout="table" columnWidths="100pt">
        <subform name="row" layout="row">
            <subform name="cell" layout="tb"><occur min="0" initial="0" max="1"/>
                <field name="value" w="100pt" h="20pt"><ui><textEdit/></ui></field>
            </subform>
        </subform>
    </subform>"#;
    assert_eq!(widgets(&render(body, Some("<form><table><row/></table></form>"))).len(), 0);
    let form = render(body, Some("<form><table><row><cell><value>present</value></cell></row></table></form>"));
    let fields = widgets(&form);
    assert_eq!(fields.len(), 1);
    assert!(fields[0].rect.h >= 20.0, "the row must measure its present cell: {:?}", fields[0]);
}

fn shrinking_section(mode: &str, constraints: &str, height: &str) -> String {
    format!(
        r#"
        <field name="intro" w="300pt" h="300pt"><ui><textEdit/></ui></field>
        <subform name="section" layout="{mode}" w="300pt" {height}>
            {constraints}
            <field name="first" w="300pt" h="180pt"><ui><textEdit/></ui></field>
            <field name="second" w="300pt" h="180pt"><ui><textEdit/></ui></field>
            <field name="third" w="300pt" h="180pt"><ui><textEdit/></ui></field>
            <subform name="optional" w="300pt" h="240pt"><occur min="0" initial="0" max="1"/>
                <field name="extra" w="300pt" h="240pt"><ui><textEdit/></ui></field>
            </subform>
        </subform>"#
    )
}

fn first_field_page(form: &Form) -> usize {
    form.pages.iter().position(|page| page.items.iter().any(|item| matches!(item, Item::Widget(field) if field.som.ends_with("first[0]")))).unwrap()
}

#[test]
fn removing_the_last_optional_row_does_not_push_a_flowing_section_to_the_next_page() {
    for mode in ["lr-tb", "rl-tb"] {
        let body = shrinking_section(mode, "", "");
        let populated = render(&body, Some("<form><section><optional/></section></form>"));
        let empty = render(&body, Some("<form><section/></form>"));
        assert_eq!(first_field_page(&populated), 0, "a section taller than the page starts in the space left");
        assert_eq!(first_field_page(&empty), 0, "shrinking below a full page must not move the whole section: {mode}");
        assert_eq!(empty.pages.len(), 2);
        for field in widgets(&empty) {
            assert!(field.rect.y >= 36.0 && field.rect.bottom() <= 756.01, "a field is kept whole at the break: {field:?}");
        }
    }
}

#[test]
fn explicitly_kept_and_fixed_height_horizontal_sections_still_move_whole() {
    for mode in ["lr-tb", "rl-tb"] {
        for constraint in [r#"<keep intact="contentArea"/>"#, r#"<keep intact="pageArea"/>"#] {
            let form = render(&shrinking_section(mode, constraint, ""), Some("<form><section/></form>"));
            assert_eq!(first_field_page(&form), 1);
        }
        let form = render(&shrinking_section(mode, "", r#"h="540pt""#), Some("<form><section/></form>"));
        assert_eq!(first_field_page(&form), 1);
    }
}
