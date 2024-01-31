    module std_gt_WIDTH :
        input left : UInt<WIDTH>
        input right : UInt<WIDTH>
        output out : UInt<1>

        out <= gt(left, right)
