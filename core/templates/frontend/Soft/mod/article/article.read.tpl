<!--buffer:thumb:0--><div class="{float}">{thumb}</div><!--buffer-->
<div class="cont">
    <div class="conttitle">
        <h1>{title}</h1>
    </div>
    <div class="conttext">
        <!--if:action:yes-->
        <div class="conttext">
            <ol>{action}</ol>
        </div>
        <!--if-->
        {content}
        <div class="clear"></div>
        {textnotice}
        <!--if:down:yes-->
        <div class="notice file">
            <span>&#9660;</span><p class="msg">{downlang}: {down}</p>
        </div>
        <!--if-->
    </div>
    <div class="conttext">
        <div>{pages}</div>
    </div>
    <div class="continfo">
        <!-- {public}: -->{time} <i>&#8260;</i> {hits}: {counts}<!--if:print:yes--> <i>&#8260;</i> {print}<!--if-->
    </div>
    <div class="continfo">
        <!--if:social:yes-->
        <div class="left social">
            {social}
        </div>
        <div class="clear"></div>
        <!--if-->
    </div>
</div>
{search} 
<!--if:media:yes-->
<div class="cont">
    <div class="site-title">
        {mediatitle}
    </div>
    <div class="conttext"> 
        {mediadesc}{mediatext}
    </div>
    <div class="clear"></div>
</div>
<div id="content">
    {media}
</div>
<div class="clear-line"></div>
<!--if--> 
<div class="clear-line"></div>
