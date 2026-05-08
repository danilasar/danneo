<table class="ac wpc_100">
<tr>
    <td class="tableod">
        <!--if:lightbox:yes-->
        <div id="image">
            <a href="{site_url}/{image}" title="{alt}">
            <!--if-->
                <!--if:image:yes-->
                <img src="{site_url}/{image}" alt="{alt}"{imagesize} />
                <!--if-->
            <!--if:lightbox:yes-->
            </a>
        </div>
        <!--if-->
        <!--if:video:yes-->
        <object>
            <embed src="{site_url}/up/mediaplayer.swf" allowscriptaccess="always" allowfullscreen="true" flashvars="file={video}&amp;searchbar=false" width="400" height="350"></embed>
        </object>
        <!--if-->
        </td>
    </tr> 
</table> 
<table class="cont-move">
<tr>
    <td class="tablein va ar wpc_10">{goback}</td>
    <td class="tablein ac"><strong>{count}</strong></td>
    <td class="tablein va al wpc_10">{next}</td>
</tr> 
</table> 
<div class="clear-line"></div>
<div id="cont-details">
<table class="details">
    <tr>
        <th class="details-one">{all_name}:</th>
        <th class="details-two"><span class="colorize">{title}</span></th>
    </tr>
    <tr>
        <td class="details-one">{all_data}:</td>
        <td class="details-two">{data}</td>
    </tr>
    <tr>
       <td class="details-one">{descript}:</td>
        <td class="details-two">{text}</td>
    </tr>
    <!--if:rate:yes-->
    <tr>
        <td class="details-one va">{all_rating}:</td>
        <td class="details-two">
            <div id="view-rate">{valrate} {rating} / {totalrating} ( <span class="colorize">{countrating}</span> )</div>
            <div id="view-progress" style="display:none"><img src="{site_url}/temp/{site_temp}/images/progress.gif" alt="" /></div>
            <!--if:formrate:yes-->{formrate}<!--if-->
        </td>
    </tr>
    <!--if-->
    <tr>
        <td class="details-one">{langhits}:</td>
        <td class="details-two"><strong>{hits}</strong></td>
    </tr>
    <tr>
        <td class="details-one va">{directlink}:</a>
        </td>
        <td class="details-two">
            <input value="{link}" class="width ccode" readonly="readonly" type="text" />
        </td>
    </tr>
    <tr>
        <td class="details-one va">{htmlcode}:</td>
        <td class="details-two va">
            <textarea class="width ccode" value="" readonly="readonly">&lt;a href=&quot;{link}&quot; target=&quot;_blank&quot;&gt;&lt;img src=&quot;{imgurl}&quot; border=&quot;0&quot; alt=&quot;&quot; /&gt;&lt;/a&gt;</textarea>
        </td>
    </tr>
    <tr>
        <td class="details-one va">{bbcode}:</td>
        <td class="details-two">
            <textarea class="width ccode" value="" readonly="readonly">[url={link}][img]{imgurl}[/img][/url]"</textarea>
        </td>
    </tr>  
    <!--if:social:yes-->
    <tr>
        <td class="details-one none va">{bookmark}:</td>
        <td class="details-two none"><div class="social">{social}</div></td>
    </tr>  
    <!--if-->
</table>
</div>
<table class="cont-move">
    <tr>
        <td class="tablein va ar wpc_10">{goback}</td>
        <td class="tablein ac">{view}</td>
        <td class="tablein va al wpc_10">{next}</td>
    </tr>
    <tr>
        <td colspan="3" class="tablein ac"><strong>{count}</strong></td>
    </tr>
</table>
