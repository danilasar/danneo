<!--buffer:thumb:0--><div class="{float}">{thumb}</div><!--buffer-->
<div class="cont">
    <div class="conttitle">
        <h1>{title}</h1>
    </div>
    <div class="conttext">
        {image}{text}
        {textmore}
        <div class="clear"></div>
        {textnotice}
    </div>
    <!--if:video:yes-->
    <div class="conttext ac">
        <object>
            <embed src="{url}/up/mediaplayer.swf" allowscriptaccess="always" allowfullscreen="true" flashvars="file={video}&amp;searchbar=false" width="400" height="350"></embed>
        </object>
    </div>
    <!--if-->
    <div class="continfo">
        <!-- {public}: --> {time} <i>&#8260;</i> {hits}: {counts} <i>&#8260;</i> <!--if:print:yes-->{print}<!--if-->
    </div>
    <div class="continfo">
        <!--if:social:yes-->
        <div class="left social">{social}</div>
        <!--if-->
        {langtags} {tags}
        <div class="clear"></div>
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
{recommend} 

