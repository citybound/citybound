(function() {var implementors = {};
implementors["chunky"] = [{text:"impl&lt;T:&nbsp;<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/fmt/trait.Display.html\" title=\"trait core::fmt::Display\">Display</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;T&gt; for <a class=\"struct\" href=\"chunky/struct.Ident.html\" title=\"struct chunky::Ident\">Ident</a>",synthetic:false,types:["chunky::Ident"]},];
implementors["compact"] = [{text:"impl&lt;T:&nbsp;<a class=\"trait\" href=\"compact/trait.Compact.html\" title=\"trait compact::Compact\">Compact</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a>, A:&nbsp;Allocator&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;T&gt;&gt; for <a class=\"struct\" href=\"compact/struct.CVec.html\" title=\"struct compact::CVec\">CompactVec</a>&lt;T, A&gt;",synthetic:false,types:["compact::compact_vec::CompactVec"]},{text:"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/alloc/string/struct.String.html\" title=\"struct alloc::string::String\">String</a>&gt; for <a class=\"struct\" href=\"compact/struct.CString.html\" title=\"struct compact::CString\">CompactString</a>",synthetic:false,types:["compact::compact_str::CompactString"]},];

            if (window.register_implementors) {
                window.register_implementors(implementors);
            } else {
                window.pending_implementors = implementors;
            }
        
})()
