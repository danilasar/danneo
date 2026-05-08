<!--if:ajax:yes-->
<script>
$(document).ready(function() {
    $('#poll-form').submit(function() {
        $('#pollsendbox').show();
        $("#pollerrorbox").html('');
        var value = $(this).serialize();
        $.ajax({
            cache:false,
            type:'POST',
            url:'{site_url}/index.php?dn=poll&re=add&ajax=1',
            data:value,
            error: function(data) { $('#poll-form').submit(); },
            success: function(data) {
                $("#pollsendbox").hide();
                if (data.match(/^<!--pollok ([0-9]+)-->/)) {
                    $("#pollajaxbox").html(data);
                } else {
                    $("#pollerrorbox").html(data);
                }
            }
        })
        return false;
    });
});
</script>
<!--if-->   
<form action="{site_url}/index.php?dn=poll" method="post" id="poll-form">
<div id="pollajaxbox">
    <div class="clear-line"></div>
    <div id="pollerrorbox"></div>
    <div id="pollsendbox" style="display:none;" class="infos">
        <img src="{site_url}/temp/{site_temp}/images/progress.gif" alt="{all_sends}" /> <strong>{all_sends} ... </strong>
    </div>
    <input name="id" value="{id}" type="hidden" />
    <input name="re" value="add" type="hidden" />
    <div class="cont">
        <div class="conttext">{desc}</div>
        <div class="conttext ac">
            <table class="poll">
                {percent}
            </table>
        </div>
        <div class="conttext ac">
            <button type="submit" id="poll-button" class="sub">{button}</button>
        </div>
    </div>
</div> 
</form>
<!--buffer:percent:0-->
<tr>
    <td class="blue wpc_25">{radio} {val_name}</td>
    <td class="gray wpc_15">{val_voc}</td>
    <td class="wpc_45">
        <div class="pollbarout" style="border-color: {val_color};">
            <div class="pollbar" style="background-color: {val_color}; width: {val_line};"></div>
        </div>
    </td>
    <td class="gray wpc_15">{val_perc} %</td>
</tr>
<!--buffer-->
