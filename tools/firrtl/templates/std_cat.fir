    module std_cat_LEFT_WIDTH_RIGHT_WIDTH_OUT_WIDTH :
        input left : UInt<LEFT_WIDTH>
        input right : UInt<RIGHT_WIDTH>
        output out : UInt<OUT_WIDTH>

        connect out, cat(left, right)