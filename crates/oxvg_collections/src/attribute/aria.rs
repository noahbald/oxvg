//! ARIA attribute types as specified in [ARIA](https://www.w3.org/TR/wai-aria-1.1/)
use crate::enum_attr;

use super::core::NonWhitespace;

enum_attr!(
    #[derive(Default)]
    /// Indicates whether inputting text could trigger display of one or more predictions of the user's intended value for an input and specifies how predictions would be presented if they are made.
    ///
    /// [ARIA](https://www.w3.org/TR/wai-aria-1.1/#aria-autocomplete)
    AriaAutocomplete {
        /// When a user is providing input, text suggesting one way to complete the provided input may be dynamically inserted after the caret.
        Inline: "inline",
        /// When a user is providing input, an element containing a collection of values that could complete the provided input may be displayed.
        List: "list",
        /// When a user is providing input, an element containing a collection of values that could complete the provided input may be displayed. If displayed, one value in the collection is automatically selected, and the text needed to complete the automatically selected value appears after the caret in the input.
        Both: "both",
        #[default]
        /// When a user is providing input, an automatic suggestion that attempts to predict how the user intends to complete the input is not displayed.
        None: "none",
    }
);
enum_attr!(
    #[derive(Default)]
    /// Indicates the element that represents the current item within a container or set of related elements.
    ///
    /// [ARIA](https://www.w3.org/TR/wai-aria-1.1/#aria-current)
    AriaCurrent {
        /// Represents the current page within a set of pages.
        Page: "page",
        /// https://www.w3.org/TR/wai-aria-1.1/#aria-current
        Step: "step",
        /// Represents the current location within an environment or context.
        Location: "location",
        /// Represents the current date within a collection of dates.
        Date: "date",
        /// Represents the current time within a set of times.
        Time: "time",
        /// Represents the current item within a set.
        True: "true",
        #[default]
        /// Does not represent the current item within a set.
        False: "false",
    }
);
enum_attr!(
    #[derive(Default)]
    /// Indicates what functions can be performed when a dragged object is released on the drop target.
    ///
    /// [ARIA](https://www.w3.org/TR/wai-aria-1.1/#aria-dropeffect)
    AriaDropEffect {
        /// A duplicate of the source object will be dropped into the target.
        Copy: "copy",
        /// A function supported by the drop target is executed, using the drag source as an input.
        Execute: "execute",
        /// A reference or shortcut to the dragged object will be created in the target object.
        Link: "link",
        /// The source object will be removed from its current location and dropped into the target.
        Move: "move",
        #[default]
        /// No operation can be performed; effectively cancels the drag operation if an attempt is made to drop on this object. Ignored if combined with any other token value. e.g., 'none copy' is equivalent to a 'copy' value.
        None: "none",
        /// There is a popup menu or dialog that allows the user to choose one of the drag operations (copy, move, link, execute) and any other drag functionality, such as cancel.
        Popup: "popup",
    }
);
enum_attr!(
    #[derive(Default)]
    /// Indicates the availability and type of interactive popup element, such as menu or dialog, that can be triggered by an element.
    ///
    /// [ARIA](https://www.w3.org/TR/wai-aria-1.1/#aria-haspopup)
    AriaHasPopup {
        #[default]
        /// Indicates the element does not have a popup.
        False: "false",
        /// Indicates the popup is a menu.
        True: "true",
        /// Indicates the popup is a menu.
        Menu: "menu",
        /// Indicates the popup is a listbox.
        Listbox: "listbox",
        /// Indicates the popup is a tree.
        Tree: "tree",
        /// Indicates the popup is a grid.
        Grid: "grid",
        /// Indicates the popup is a dialog.
        Dialog: "dialog",
    }
);
enum_attr!(
    #[derive(Default)]
    /// Indicates the entered value does not conform to the format expected by the application.
    ///
    /// [ARIA](https://www.w3.org/TR/wai-aria-1.1/#aria-invalid)
    AriaInvalid {
        /// A grammatical error was detected.
        Grammar: "grammar",
        /// There are no detected errors in the value.
        #[default]
        False: "false",
        /// A spelling error was detected.
        Spelling: "spelling",
        /// The value entered by the user has failed validation.
        True: "true",
    }
);
enum_attr!(
    #[derive(Default)]
    /// Indicates that an element will be updated, and describes the types of updates the user agents, assistive technologies, and user can expect from the live region.
    ///
    /// [ARIA](https://www.w3.org/TR/wai-aria-1.1/#aria-live)
    AriaLive {
        /// Indicates that updates to the region have the highest priority and should be presented the user immediately.
        Assertive: "assertive",
        /// Indicates that updates to the region should not be presented to the user unless the used is currently focused on that region.
        Off: "off",
        #[default]
        /// Indicates that updates to the region should be presented at the next graceful opportunity, such as at the end of speaking the current sentence or when the user pauses typing.
        Polite: "polite",
    }
);
enum_attr!(
    /// Indicates whether the element's orientation is horizontal, vertical, or unknown/ambiguous.
    ///
    /// [ARIA](https://www.w3.org/TR/wai-aria-1.1/#aria-orientation)
    AriaOrientation {
        /// The element is oriented horizontally.
        Horizontal: "horizontal",
        /// The element's orientation is unknown/ambiguous.
        Undefined: "undefined",
        /// The element is oriented vertically.
        Vertical: "vertical",
    }
);
enum_attr!(
    /// Indicates what notifications the user agent will trigger when the accessibility tree within a live region is modified.
    AriaRelevant {
        /// Element nodes are added to the accessibility tree within the live region.
        Addition: "additions",
        /// Equivalent to the combination of values, "additions text".
        AdditionsText: "additions text",
        /// Equivalent to the combination of all values, "additions removals text".
        All: "all",
        /// Text content, a text alternative, or an element node within the live region is removed from the accessibility tree.
        Removals: "removals",
        /// Text content or a text alternative is added to any descendant in the accessibility tree of the live region.
        Text: "text",
    }
);
enum_attr!(
    #[derive(Default)]
    /// Indicates if items in a table or grid are sorted in ascending or descending order.
    ///
    /// [ARIA](https://www.w3.org/TR/wai-aria-1.1/#aria-sort)
    AriaSort {
        /// Items are sorted in ascending order by this column.
        Ascending: "ascending",
        /// Items are sorted in descending order by this column.
        Descending: "descending",
        /// There is no defined sort applied to the column.
        #[default]
        None: "none",
        /// A sort algorithm other than ascending or descending has been applied.
        Other: "other",
    }
);
/// A reference to the ID of another element in the same document
///
/// [ARIA](https://www.w3.org/TR/wai-aria-1.1/#valuetype_idref)
pub type IDReference<'i> = NonWhitespace<'i>;
enum_attr!(
    /// Ontological roles
    ///
    /// [ARIA](https://www.w3.org/TR/wai-aria-1.1/#role_definitions)
    Role {
        /// A type of live region with important, and usually time-sensitive, information.
        Alert: "alert",
        /// A type of dialog that contains an alert message, where initial focus goes to an element within the dialog.
        AlertDialog: "alertdialog",
        /// A structure containing one or more focusable elements requiring user input that do not follow a standard interaction pattern supported by a widget role.
        Application: "aplication",
        /// A section of a page that consists of a composition that forms an independent part of a document, page, or site.
        Article: "article",
        /// A region that contains mostly site-oriented content, rather than page-specific content.
        Banner: "banner",
        /// An input that allows for user-triggered actions when clicked or pressed.
        Button: "button",
        /// A cell in a tabular container.
        Cell: "cell",
        /// A checkable input that has three possible values: true, false, or mixed.
        Checkbox: "checkbox",
        /// A cell containing header information for a column.
        ColumnHeader: "columnheader",
        /// A composite widget containing a single-line textbox and another element that can dynamically pop up to help the user set the value of the textbox.
        Combobox: "combobox",
        /// A form of widget that performs an action but does not receive input data.
        Command: "command",
        /// A supporting section of the document, designed to be complementary to the main content at a similar level in the DOM hierarchy, but remains meaningful when separated from the main content.
        Complementary: "complementary",
        /// A widget that may contain navigable descendants or owned children.
        Composite: "composite",
        /// A large perceivable region that contains information about the parent document.
        ContentInfo: "contentinfo",
        /// A definition of a term or concept.
        Definition: "definition",
        /// A dialog is a descendant window of the primary window of a web application.
        Dialog: "dialog",
        /// A list of references to members of a group, such as a static table of contents.
        Directory: "directory",
        /// An element containing content that assistive technology users may want to browse in a reading mode.
        Document: "document",
        /// A scrollable list of articles where scrolling may cause articles to be added to or removed from either end of the list.
        Feed: "feed",
        /// A perceivable section of content that typically contains a graphical document, images, code snippets, or example text.
        Figure: "figure",
        /// A landmark region that contains a collection of items and objects that, as a whole, combine to create a form. See related search.
        Form: "form",
        /// A composite widget containing a collection of one or more rows with one or more cells where some or all cells in the grid are focusable by using methods of two-dimensional navigation, such as directional arrow keys.
        Grid: "grid",
        /// A cell in a grid or treegrid.
        Gridcell: "gridcell",
        /// A set of user interface objects which are not intended to be included in a page summary or table of contents by assistive technologies.
        Group: "group",
        /// A heading for a section of the page.
        Heading: "heading",
        /// A container for a collection of elements that form an image.
        Img: "img",
        /// A generic type of widget that allows user input.
        Input: "input",
        /// A perceivable section containing content that is relevant to a specific, author-specified purpose and sufficiently important that users will likely want to be able to navigate to the section easily and to have it listed in a summary of the page.
        Landmark: "landmark",
        /// An interactive reference to an internal or external resource that, when activated, causes the user agent to navigate to that resource.
        Link: "link",
        /// A section containing listitem elements.
        List: "list",
        /// A widget that allows the user to select one or more items from a list of choices.
        Listbox: "listbox",
        /// A single item in a list or directory.
        Listitem: "listitem",
        /// A type of live region where new information is added in meaningful order and old information may disappear.
        Log: "log",
        /// The main content of a document.
        Main: "main",
        /// A type of live region where non-essential information changes frequently.
        Marquee: "marquee",
        /// Content that represents a mathematical expression.
        Math: "math",
        /// A type of widget that offers a list of choices to the user.
        Menu: "menu",
        /// A presentation of menu that usually remains visible and is usually presented horizontally.
        Menubar: "menubar",
        /// An option in a set of choices contained by a menu or menubar.
        Menuitem: "menuitem",
        /// A menuitem with a checkable state whose possible values are true, false, or mixed.
        MenuitemCheckbox: "menuitemcheckbox",
        /// A checkable menuitem in a set of elements with the same role, only one of which can be checked at a time.
        MenuitemRadio: "menuitemradio",
        /// A collection of navigational elements (usually links) for navigating the document or related documents.
        Navigation: "navigation",
        /// An element whose implicit native role semantics will not be mapped to the accessibility API.
        None: "none",
        /// A section whose content is parenthetic or ancillary to the main content of the resource.
        Note: "note",
        /// A selectable item in a select list.
        Option: "option",
        /// An element whose implicit native role semantics will not be mapped to the accessibility API.
        Presentation: "presentation",
        /// An element that displays the progress status for tasks that take a long time.
        ProgressBar: "progressbar",
        /// A checkable input in a group of elements with the same role, only one of which can be checked at a time.
        Radio: "radio",
        /// A group of radio buttons.
        RadioGroup: "radiogroup",
        /// An input representing a range of values that can be set by the user.
        Range: "range",
        /// A perceivable section containing content that is relevant to a specific, author-specified purpose and sufficiently important that users will likely want to be able to navigate to the section easily and to have it listed in a summary of the page.
        Region: "region",
        /// The base role from which all other roles in this taxonomy inherit.
        RoleType: "roletype",
        /// A row of cells in a tabular container.
        Row: "row",
        /// A structure containing one or more row elements in a tabular container.
        RowGroup: "rowgroup",
        /// A cell containing header information for a row in a grid.
        RowHeader: "rowheader",
        /// A graphical object that controls the scrolling of content within a viewing area, regardless of whether the content is fully displayed within the viewing area.
        Scrollbar: "scrollbar",
        /// A landmark region that contains a collection of items and objects that, as a whole, combine to create a search facility. See related form and searchbox.
        Search: "search",
        /// A type of textbox intended for specifying search criteria. See related textbox and search.
        SearchBox: "searchbox",
        /// A renderable structural containment unit in a document or application.
        Section: "section",
        /// A structure that labels or summarizes the topic of its related section.
        SectionHead: "sectionhead",
        /// A form widget that allows the user to make selections from a set of choices.
        Select: "select",
        /// A divider that separates and distinguishes sections of content or groups of menuitems.
        Separator: "separator",
        /// A user input where the user selects a value from within a given range.
        Slider: "slider",
        /// A form of range that expects the user to select from among discrete choices.
        SpinButton: "spinbutton",
        /// A type of live region whose content is advisory information for the user but is not important enough to justify an alert, often but not necessarily presented as a status bar.
        Status: "status",
        /// A document structural element.
        Structure: "structure",
        /// A type of checkbox that represents on/off values, as opposed to checked/unchecked values. See related checkbox.
        Switch: "switch",
        /// A grouping label providing a mechanism for selecting the tab content that is to be rendered to the user.
        Tab: "tab",
        /// A section containing data arranged in rows and columns.
        Table: "table",
        /// A list of tab elements, which are references to tabpanel elements.
        TabList: "tablist",
        /// A container for the resources associated with a tab, where each tab is contained in a tablist.
        TabPanel: "tabpanel",
        /// A word or phrase with a corresponding definition.
        Term: "term",
        /// A type of input that allows free-form text as its value.
        TextBox: "textbox",
        /// A type of live region containing a numerical counter which indicates an amount of elapsed time from a start point, or the time remaining until an end point.
        Timer: "timer",
        /// A collection of commonly used function buttons or controls represented in compact visual form.
        Toolbar: "toolbar",
        /// A contextual popup that displays a description for an element.
        Tooltip: "tooltip",
        /// A type of list that may contain sub-level nested groups that can be collapsed and expanded.
        Tree: "tree",
        /// A grid whose rows can be expanded and collapsed in the same manner as for a tree.
        TreeGrid: "treegrid",
        /// An option item of a tree.
        TreeItem: "treeitem",
        /// An interactive component of a graphical user interface (GUI).
        Widget: "widget",
        /// A browser or application window.
        Window: "window",
    }
);
enum_attr!(
    /// A true/false value with an intermediate "mixed" state
    ///
    /// [ARIA](https://www.w3.org/TR/wai-aria-1.1/#valuetype_tristate)
    Tristate {
        /// True
        True: "true",
        /// False
        Mixed: "mixed",
        /// Mixed
        False: "false",
    }
);
