<script>
$(document).ready(function() {
    $('#refresh').click(function() {
         var t = new Date().getTime();
         $('#divcaptcha').html('<img src="{site_url}/image.php?to=captcha&t=' + t + '" alt="Captcha" />');
    });
});
</script>
<form action="{site_url}/index.php?dn=subscribe" method="post">
<div class="forms"> 
    <fieldset class="standart">
        <legend>{subscribe_your_name}</legend>
        <strong>|</strong><input class="width" name="subname" type="text" />
    </fieldset>
    <fieldset class="standart">
        <legend>{subscribe_your_mail}</legend>
        <strong>|</strong><input class="width" name="submail" type="text" />
    </fieldset>
    <fieldset class="standart">
        <legend>{subscribe_your_format}</legend>
        <strong>|</strong>
        <select name="subformat">
            <option value="0" selected="selected">TEXT</option>
            <option value="1">HTML</option>
        </select>
    </fieldset>
    <!--if:captcha:yes-->
    <fieldset class="standart">
        <legend>Captcha</legend>
        <table class="ac wpc_100">
            <tr>
                <td class="ac pad wpc_100">
                    <strong>|</strong><input class="width" id="captcha" name="captcha" type="text" maxlength="5" />
                </td>
                <td class="ac pad">
                    <div id="divcaptcha"><img src="{site_url}/image.php?to=captcha" alt="" /></div>
                </td>
                <td class="ac pad">
                    <button type="button" id="refresh" class="sub">{all_refresh}</button>
                </td>
            </tr>
        </table>
    </fieldset>
    <!--if-->
    <!--if:control:yes-->
    <fieldset class="standart">
        <legend>{control_word}</legend>
        <p>{control}</p>
        <strong>|</strong><input class="width" id="respon" name="respon" size="30" type="text" maxlength="255" value="">
        <input name="cid" type="hidden" value="{cid}" />
    </fieldset>
    <!--if-->
    <div class="pad ac">
        <input name="to" type="hidden" value="check" />
        <button type="submit" class="sub">{subscribe_button}</button>
    </div>
    <div class="clear"></div>
</div>
</form>
