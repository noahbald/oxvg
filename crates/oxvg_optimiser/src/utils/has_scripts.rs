use oxvg_ast::element::Element;

pub fn has_scripts<E: Element>(root: &E) -> bool {
    root
            .find_element()
            .and_then(|e| e.select("script,a[href^='javascript:'],[onbegin],[onend],[onrepeat],[onload],[onabort],[onerror],[onresize],[onscroll],[onunload],[onzoom],[oncopy],[oncut],[onpaste],[oncancel],[oncanplay],[oncanplaythrough],[onchange],[onclick],[onclose],[oncuechange],[ondblclick],[ondrag],[ondragend],[ondragenter],[ondragleave],[ondragover],[ondragstart],[ondrop],[ondurationchange],[onemptied],[onended],[onfocus],[oninput],[oninvalid],[onkeydown],[onkeypress],[onkeyup],[onloadeddata],[onloadedmetadata],[onloadstart],[onmousedown],[onmouseenter],[onmouseleave],[onmousemove],[onmouseout],[onmouseup],[onmousewheel],[onpause],[onplay],[onplaying],[onprogress],[onratechange],[onreset],[onseeked],[onseeking],[onselect],[onshow],[onstalled],[onsubmit],[onsuspend],[ontimeupdate],[ontoggle],[onvolumechange],[onwaiting],[onactivate],[onfocusin],[onfocusout],[onmouseover]").ok())
            .is_some_and(|mut e| e.next().is_some())
}
