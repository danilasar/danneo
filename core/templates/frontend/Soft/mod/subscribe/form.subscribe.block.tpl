<script>
$(document).ready(function() {
    $('#refresh').click(function() {
         var t = new Date().getTime();
         $('#subcaptcha').html('<img src="{site_url}/image.php?to=captcha&t=' + t + '" alt="" />');
    });
});
</script>
<form action="{site_url}/index.php?dn=subscribe" method="post">
<fieldset class="standart">
    <legend>{subscribe_your_name}</legend>
    <input class="width" name="subname" type="text" />
</fieldset>
<fieldset class="standart">
    <legend>{subscribe_your_mail}</legend>
    <input class="width" name="submail" type="text" />
</fieldset>
<fieldset class="standart">
    <legend>{subscribe_your_format}</legend>
    <select name="subformat" class="width">
        <option value="0" selected="selected">TEXT</option>
        <option value="1">HTML</option>
    </select>
</fieldset>
<!--if:captcha:yes-->
<fieldset class="standart">
    <legend><strong>*</strong> Captcha</legend>
    <table class="wpc_100">
    <tr>
        <td class="ac va pad wpc_100">
            <input class="width" id="captcha" name="captcha" type="text" maxlength="5" />
        </td>
    </tr>
    <tr>
        <td class="ac va pad">
            <div id="subcaptcha"><img src="{site_url}/image.php?to=captcha" alt="" /></div>
        </td>
    </tr>
    <tr>
        <td class="ac va pad">
            <button type="button" id="refresh" class="sub">{all_refresh}</button>
        </td>
    </tr>
    </table>
</fieldset>
<!--if-->
<!--if:control:yes-->
<fieldset class="standart">
    <legend><strong>*</strong> {control_word}</legend>
    <p>{control}</p>
    <p><input class="width" id="respon" name="respon" size="30" type="text" /></p>
    <input name="cid" type="hidden" value="{cid}" />
</fieldset>
<!--if-->
<div class="pad ac">
     <input name="to" type="hidden" value="check" />
     <button type="submit" class="sub">{subscribe_button}</button>
</div>
<div class="clear"></div>
</form>
